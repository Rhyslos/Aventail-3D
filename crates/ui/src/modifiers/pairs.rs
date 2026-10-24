//! The modifiers made from two selected objects
//! ([decision 0147](../../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)):
//! **Bend along curve** and **Shrinkwrap onto**. As for booleans and
//! scatter, the object selected first carries the modifier: the shape is
//! clicked first, then the curve or the target is Ctrl + clicked.

use crate::commands::CommandId;
use crate::text::tr;
use scene::{
    CurveBendSettings, Document, MAX_MODIFIERS, Modifier, ModifierKind, ModifierRefusal, ObjectId,
    ObjectKind, ShrinkwrapSettings,
};

/// The words for one of the two commands.
struct Words {
    none: &'static str,
    one: &'static str,
    more: &'static str,
    wrong: &'static str,
    looped: &'static str,
}

fn words(bend: bool) -> Words {
    if bend {
        Words {
            none: tr("Select the shape first, then Ctrl + click the curve to bend it along"),
            one: tr("Ctrl + click a curve to bend along: the shape selected first is bent"),
            more: tr("Select only two: the shape to bend first, then the curve"),
            wrong: tr("The second selected isn't a curve: Ctrl + click a curve to bend along"),
            looped: tr("A shape can't be bent along itself, or along a curve made from it"),
        }
    } else {
        Words {
            none: tr("Select the shape first, then Ctrl + click the shape to wrap it onto"),
            one: tr("Ctrl + click a shape to wrap onto: the shape selected first is wrapped"),
            more: tr("Select only two: the shape to wrap first, then the target"),
            wrong: tr(
                "The second selected has no surface: groups and lattice cages can't be wrapped \
                 onto",
            ),
            looped: tr("A shape can't be wrapped onto itself, or onto a shape made from it"),
        }
    }
}

/// The shape and the modifier Bend along curve or Shrinkwrap onto
/// (`command`) would add to it: the shape selected first, reading the
/// object selected second. Or why it can't be done.
pub(crate) fn pair_plan(
    doc: &Document,
    command: CommandId,
) -> Result<(ObjectId, ModifierKind), &'static str> {
    let bend = command == CommandId::BendAlongCurve;
    let say = words(bend);
    let order = doc.selection_order();
    let (shape, other) = match order[..] {
        [] => return Err(say.none),
        [_] => return Err(say.one),
        [shape, other] => (shape, other),
        _ => return Err(say.more),
    };
    let scene = doc.scene();
    let object = scene.object(shape).ok_or(say.none)?;
    match object.kind {
        ObjectKind::Instance(_) => {
            return Err(tr(
                "The first selected is an instance: select its original, or bake it into a shape \
                 first",
            ));
        }
        _ if !object.kind.has_shape() => {
            return Err(tr(
                "The first selected has no shape of its own: select a shape first",
            ));
        }
        _ => {}
    }
    if object.modifiers.len() >= MAX_MODIFIERS {
        return Err(tr("A shape can have at most 32 modifiers"));
    }
    let kind = if bend {
        ModifierKind::BendAlongCurve(CurveBendSettings::of(other))
    } else {
        ModifierKind::Shrinkwrap(ShrinkwrapSettings::of(other))
    };
    let usable = scene
        .object(other)
        .is_some_and(|o| kind.takes_source(&o.kind));
    if !usable {
        return Err(say.wrong);
    }
    let mut after = object.modifiers.clone();
    after.push(Modifier::new(kind.clone()));
    match scene.check_modifiers(shape, &after) {
        Ok(()) => Ok((shape, kind)),
        Err(ModifierRefusal::BadSource) => Err(say.looped),
        Err(_) => Err(say.wrong),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, CurveObject, CurveShape, PrimitiveKind, ReadyCurve};

    fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        doc.selection()[0]
    }

    /// Selects `ids` one after another, as clicking the first and Ctrl +
    /// clicking the others does.
    fn pick(doc: &mut Document, ids: &[ObjectId]) {
        doc.clear_selection();
        for n in 1..=ids.len() {
            doc.set_selection(ids[..n].iter().copied());
        }
    }

    fn circle(doc: &mut Document) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
            CurveShape::Ready(ReadyCurve::Circle { radius: 50.0 }),
        ))))
        .unwrap();
        doc.selection()[0]
    }

    #[test]
    fn the_shape_selected_first_reads_the_curve_selected_second() {
        let mut doc = Document::new();
        let cube = create(&mut doc, PrimitiveKind::Cube);
        let curve = circle(&mut doc);
        let bend = CommandId::BendAlongCurve;
        pick(&mut doc, &[]);
        assert!(pair_plan(&doc, bend).is_err());
        pick(&mut doc, &[cube]);
        assert!(pair_plan(&doc, bend).is_err(), "no curve yet");
        pick(&mut doc, &[cube, curve]);
        let (shape, kind) = pair_plan(&doc, bend).unwrap();
        assert_eq!((shape, kind.sources()), (cube, vec![curve]));
        // The other way round: a curve can't be bent along a cube.
        pick(&mut doc, &[curve, cube]);
        assert!(pair_plan(&doc, bend).is_err());
        // Three is too many.
        let sphere = create(&mut doc, PrimitiveKind::Sphere);
        pick(&mut doc, &[cube, curve, sphere]);
        assert!(pair_plan(&doc, bend).is_err());
        // The command adds it.
        pick(&mut doc, &[cube, curve]);
        let add = crate::modifiers::command(&doc, bend).unwrap();
        doc.execute(add).unwrap();
        assert!(matches!(
            doc.scene().modifiers(cube)[0].kind,
            ModifierKind::BendAlongCurve(_)
        ));
    }

    #[test]
    fn the_shape_selected_first_is_wrapped_onto_the_second() {
        let mut doc = Document::new();
        let plane = create(&mut doc, PrimitiveKind::Plane);
        let cube = create(&mut doc, PrimitiveKind::Cube);
        let wrap = CommandId::Shrinkwrap;
        pick(&mut doc, &[cube, plane]);
        let (shape, kind) = pair_plan(&doc, wrap).unwrap();
        assert_eq!((shape, kind.sources()), (cube, vec![plane]));
        doc.execute(crate::modifiers::command(&doc, wrap).unwrap())
            .unwrap();
        // The other way round would be a loop.
        pick(&mut doc, &[plane, cube]);
        assert_eq!(pair_plan(&doc, wrap).unwrap_err(), words(false).looped);
        // A group has no surface; a group first has no shape.
        doc.execute(Command::Create(ObjectKind::Group)).unwrap();
        let group = doc.selection()[0];
        pick(&mut doc, &[cube, group]);
        assert_eq!(pair_plan(&doc, wrap).unwrap_err(), words(false).wrong);
        pick(&mut doc, &[group, cube]);
        assert!(pair_plan(&doc, wrap).is_err());
    }
}
