# Opens each scene Godot imported from the models folder and checks it:
# every file holds at least one mesh, and the file with Godot's collision
# names (godot-collision.glb) gets its collision shapes, as does the file
# with collision in parts (godot-collision-parts.glb), and the levels of
# detail (godot-lods.glb, imported with lod_ranges.gd) are shown between
# their distances and have their real shapes (decision 0165). Run by
# godot-import.sh after the import; prints "FAILED" lines for problems.
extends SceneTree


func _init() -> void:
	var failed := false
	var dir := DirAccess.open("res://models")
	if dir == null:
		print("FAILED: no models folder")
		quit(1)
		return
	for name in dir.get_files():
		if not name.ends_with(".glb"):
			continue
		var packed: PackedScene = load("res://models/" + name)
		if packed == null:
			print("FAILED: %s didn't load" % name)
			failed = true
			continue
		var root := packed.instantiate()
		var meshes := count(root, "MeshInstance3D")
		var shapes := count(root, "CollisionShape3D")
		print("%s: %d meshes, %d collision shapes" % [name, meshes, shapes])
		if meshes == 0:
			print("FAILED: %s has no meshes" % name)
			failed = true
		# Cube-col, Sphere-convcol and the cylinder's box each become one.
		if name == "godot-collision.glb" and shapes < 3:
			print("FAILED: %s should have 3 collision shapes" % name)
			failed = true
		# The arch's and the ring's parts (decision 0151) each become one,
		# and aren't drawn: only the two shapes are meshes.
		if name == "godot-collision-parts.glb" and (shapes < 5 or meshes != 2):
			print("FAILED: %s should have 2 meshes and 5 or more collision shapes" % name)
			failed = true
		if name == "godot-lods.glb" and not (lods_ranged(root) and lods_shaped(root)):
			failed = true
		root.free()
	if not failed:
		print("All scenes checked")
	quit(1 if failed else 0)


# Whether the sphere and the ring of godot-lods.glb came in with three
# levels each, shown one after another from 10 m, 20 m and 40 m (the
# standard distances), by the post-import script.
func lods_ranged(root: Node) -> bool:
	var ok := true
	for shape in ["Sphere", "Torus"]:
		var begins := [0.0, 10.0, 20.0, 40.0]
		var ends := [10.0, 20.0, 40.0, 0.0]
		for level in 4:
			var node_name: String = shape if level == 0 else "%s_LOD%d" % [shape, level]
			var node := root.find_child(node_name, true, false) as GeometryInstance3D
			if node == null:
				print("FAILED: godot-lods.glb has no %s" % node_name)
				ok = false
				continue
			if not node.visible or not is_equal_approx(node.visibility_range_begin, begins[level]) \
					or not is_equal_approx(node.visibility_range_end, ends[level]):
				print("FAILED: %s is shown from %s to %s (visible %s)" % [node_name,
						node.visibility_range_begin, node.visibility_range_end, node.visible])
				ok = false
	return ok


# Whether godot-lods.glb's levels are their real shapes (decision 0165):
# the sphere's first level, without its Subdivide, has far fewer triangles
# than the smoothed sphere, its second (smoothed once) fewer than it, and
# the ring's hand-made second level came in as that level only, never as
# a mesh of its own ("Torus LOD2"), with the slice taken out of it.
func lods_shaped(root: Node) -> bool:
	var ok := true
	var sphere := triangles(root, "Sphere")
	var first := triangles(root, "Sphere_LOD1")
	var second := triangles(root, "Sphere_LOD2")
	if not (first * 4 <= sphere and second < sphere):
		print("FAILED: the sphere's levels have %d and %d triangles, the sphere %d" % [first,
				second, sphere])
		ok = false
	if root.find_child("Torus LOD2", true, false) != null \
			or root.find_child("Torus_LOD2", true, false) == null:
		print("FAILED: the ring's hand-made level should come in as Torus_LOD2 only")
		ok = false
	if triangles(root, "Torus_LOD2") >= triangles(root, "Torus"):
		print("FAILED: the ring's hand-made level should have a slice taken out")
		ok = false
	print("godot-lods.glb: Sphere %d, LOD1 %d, LOD2 %d triangles; Torus %d, LOD2 %d" % [sphere,
			first, second, triangles(root, "Torus"), triangles(root, "Torus_LOD2")])
	return ok


# The triangles of the mesh of node `name` under `root` (0 without one).
func triangles(root: Node, name: String) -> int:
	var node := root.find_child(name, true, false) as MeshInstance3D
	if node == null or node.mesh == null:
		return 0
	return node.mesh.get_faces().size() / 3


# How many nodes of `kind` are in the tree under `node`, itself included.
func count(node: Node, kind: String) -> int:
	var n := 1 if node.is_class(kind) else 0
	for child in node.get_children():
		n += count(child, kind)
	return n
