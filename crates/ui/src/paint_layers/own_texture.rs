//! Each model its own texture ([decision 0232](../../../../docs/decisions/0232-each-model-its-own-texture.md)).
//!
//! A texture belongs to a material, and every new shape starts with the
//! same standard material, so two shapes painted one after the other
//! would share one texture, their UVs on top of each other. Painting a
//! shape whose standard material another shape also has first gives it
//! its own copy, named after it, with a copy of any painting: one undo
//! step, said in the status bar. A material made and given to several
//! shapes on purpose (a trim sheet, a kit) stays shared; the shared-paint
//! note (decision 0214) still warns when those overlap.

use crate::text::tr;
use scene::{Command, Document, Material, ObjectId, ObjectKind, Scene};

/// `object`'s materials: the standard one when it has none of its own
/// (every polygon then uses it).
pub(crate) fn materials_of(object: &scene::Object) -> Vec<Material> {
    if object.materials.is_empty() {
        vec![Material::standard()]
    } else {
        object.materials.clone()
    }
}

/// Whether a shape of this kind has its own polygons to paint (an
/// instance shows its original's).
pub(crate) fn paints(kind: &ObjectKind) -> bool {
    matches!(kind, ObjectKind::Primitive(_) | ObjectKind::Mesh(_))
}

/// Whether painting `owner`'s `material` should first give it its own
/// copy: `material` is the standard one, and another shape (not an
/// instance of `owner`, which shows its materials) has it too.
pub(crate) fn needs_own(scene: &Scene, owner: ObjectId, material: &str) -> bool {
    if material != Material::standard().name {
        return false;
    }
    let Some(object) = scene.object(owner) else {
        return false;
    };
    if !paints(&object.kind) || !materials_of(object).iter().any(|m| m.name == material) {
        return false;
    }
    scene.walk().iter().any(|(id, _)| {
        *id != owner
            && scene.object(*id).is_some_and(|other| {
                paints(&other.kind) && materials_of(other).iter().any(|m| m.name == material)
            })
    })
}

/// A material name no shape in `scene` uses and no texture set has: the
/// object's own name for the standard material, else "`material` –
/// `object`"; numbered if taken.
pub(crate) fn unused_name(scene: &Scene, object: &str, material: &str) -> String {
    let taken = |name: &str| {
        scene.paint().set(name).is_some()
            || scene.walk().iter().any(|(id, _)| {
                scene
                    .object(*id)
                    .is_some_and(|o| materials_of(o).iter().any(|m| m.name == name))
            })
    };
    let base = if material == Material::standard().name && !object.trim().is_empty() {
        object.trim().to_string()
    } else {
        crate::trf!("{} – {}", material, object)
    };
    std::iter::once(base.clone())
        .chain((2..).map(|n| format!("{base} {n}")))
        .find(|name| !taken(name))
        .unwrap_or(base)
}

/// Gives `owner` its own copy of `material` (its colour and settings,
/// and a copy of its painting) as one undo step, merged under `key`.
/// The new material's name, and what to say in the status bar.
pub(crate) fn give_own(
    doc: &mut Document,
    owner: ObjectId,
    material: &str,
    key: u64,
) -> Result<(String, String), String> {
    let scene = doc.scene();
    let object = scene
        .object(owner)
        .ok_or_else(|| tr("That object is gone.").to_string())?;
    let name = unused_name(scene, &object.name, material);
    let mut materials = materials_of(object);
    for m in &mut materials {
        if m.name == material {
            m.name = name.clone();
        }
    }
    let mut commands = vec![Command::SetMaterials {
        id: owner,
        materials,
    }];
    if let Some(set) = scene.paint().set(material) {
        commands.push(Command::SetTextureSet {
            material: name.clone(),
            set: Some((**set).clone()),
        });
    }
    let object_name = object.name.clone();
    let label = crate::trf!("Give {} its own texture", object_name);
    doc.execute_all_labelled(commands, &label, key)
        .map_err(|why| why.to_string())?;
    let said = crate::trf!(
        "{} now has its own material, {}, and so its own texture: paint on it stays on it. \
         Ctrl + Z takes it back.",
        object_name,
        name
    );
    Ok((name, said))
}

/// [`give_own`] when [`needs_own`] says so; the material to paint on
/// either way. `note` gets what to say, or why it couldn't.
pub(crate) fn own_if_shared(
    doc: &mut Document,
    owner: ObjectId,
    material: &str,
    key: u64,
    note: &mut Option<String>,
) -> String {
    if !needs_own(doc.scene(), owner, material) {
        return material.to_string();
    }
    match give_own(doc, owner, material, key) {
        Ok((name, said)) => {
            *note = Some(said);
            name
        }
        Err(why) => {
            *note = Some(why);
            material.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::PrimitiveKind;

    fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        doc.selection()[0]
    }

    #[test]
    fn a_lone_shape_keeps_the_standard_material() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        assert!(!needs_own(doc.scene(), a, &Material::standard().name));
    }

    #[test]
    fn a_second_shape_with_the_standard_material_gets_its_own_named_after_it() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        let _b = create(&mut doc, PrimitiveKind::Sphere);
        let standard = Material::standard().name;
        assert!(needs_own(doc.scene(), a, &standard));
        let mut note = None;
        let name = own_if_shared(&mut doc, a, &standard, 7, &mut note);
        let cube_name = doc.scene().object(a).unwrap().name.clone();
        assert_eq!(name, cube_name);
        assert_eq!(doc.scene().material_list(a)[0].name, name);
        assert!(note.unwrap().contains(&name));
        // The other one is the only one with the standard material now.
        assert!(!needs_own(doc.scene(), a, &name));
        // One undo step puts it back.
        doc.undo().unwrap();
        assert_eq!(doc.scene().material_list(a)[0].name, standard);
    }

    #[test]
    fn the_painting_comes_along_and_names_dont_clash() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        let b = create(&mut doc, PrimitiveKind::Cube);
        let standard = Material::standard().name;
        let mut set = paint::TextureSet::new(64).unwrap();
        set.layers.push(paint::Layer::fill("Red", [255, 0, 0, 255]));
        doc.execute(Command::SetTextureSet {
            material: standard.clone(),
            set: Some(set.clone()),
        })
        .unwrap();
        // Name a material after the first cube, so the copy is numbered.
        let cube = doc.scene().object(a).unwrap().name.clone();
        let mut named = Material::standard();
        named.name = cube.clone();
        doc.execute(Command::SetMaterials {
            id: b,
            materials: vec![Material::standard(), named],
        })
        .unwrap();
        let mut note = None;
        let name = own_if_shared(&mut doc, a, &standard, 9, &mut note);
        assert_eq!(name, format!("{cube} 2"));
        assert_eq!(**doc.scene().paint().set(&name).unwrap(), set);
        assert!(doc.scene().paint().set(&standard).is_some(), "b keeps it");
    }

    #[test]
    fn materials_shared_on_purpose_and_instances_stay_as_they_are() {
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        let b = create(&mut doc, PrimitiveKind::Cube);
        let mut trim = Material::standard();
        trim.name = "Trim".into();
        for id in [a, b] {
            doc.execute(Command::SetMaterials {
                id,
                materials: vec![trim.clone()],
            })
            .unwrap();
        }
        assert!(!needs_own(doc.scene(), a, "Trim"));
        // An instance shows its original's materials: no copy for it.
        let mut doc = Document::new();
        let a = create(&mut doc, PrimitiveKind::Cube);
        doc.set_selection([a]);
        doc.execute(Command::Duplicate {
            ids: vec![a],
            instances: true,
        })
        .unwrap();
        let standard = Material::standard().name;
        assert!(!needs_own(doc.scene(), a, &standard));
        // Unknown objects and missing materials ask for nothing.
        assert!(!needs_own(doc.scene(), ObjectId::from_raw(999), &standard));
        assert!(!needs_own(doc.scene(), a, "Nothing"));
    }
}
