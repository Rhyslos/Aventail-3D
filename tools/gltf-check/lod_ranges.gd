@tool
extends EditorScenePostImport

# Turns the levels of detail exported from the app (decision 0155) into
# visibility ranges: each node with "lod_begin" and "lod_end" in its glTF
# extras is shown between those distances (in metres; an end of 0 is no
# end), and shown at all.
#
# The user manual (docs/manual/levels-of-detail.md) prints this script
# for people to copy into their Godot project: keep the two the same.
# godot-import.sh imports godot-lods.glb with it, and check-scenes.gd
# checks the result.


func _post_import(scene: Node) -> Object:
	_set_ranges(scene)
	return scene


func _set_ranges(node: Node) -> void:
	if node is GeometryInstance3D and node.has_meta("extras"):
		var extras: Dictionary = node.get_meta("extras")
		if extras.has("lod_begin") and extras.has("lod_end"):
			node.visibility_range_begin = extras["lod_begin"]
			node.visibility_range_end = extras["lod_end"]
			node.visible = true
	for child in node.get_children():
		_set_ranges(child)
