# Opens each scene Godot imported from the models folder and checks it:
# every file holds at least one mesh, and the file with Godot's collision
# names (godot-collision.glb) gets its collision shapes. Run by
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
		root.free()
	if not failed:
		print("All scenes checked")
	quit(1 if failed else 0)


# How many nodes of `kind` are in the tree under `node`, itself included.
func count(node: Node, kind: String) -> int:
	var n := 1 if node.is_class(kind) else 0
	for child in node.get_children():
		n += count(child, kind)
	return n
