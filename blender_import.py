from __future__ import annotations

import ctypes
import os
import platform
import re
import time
from typing import Optional

import bpy  # type: ignore
from lupa import LuaRuntime
from dataclasses import dataclass
from dataclasses_json import DataClassJsonMixin
from mathutils import Matrix  # type: ignore

# Configuration
VERIFY_SIGNATURE = True  # Verify the library signature. DISABLING THIS IS NOT RECOMMENDED!
ARMATURE_FULL = False  # Use all Nodes for creating the armature, not just Joint Nodes.
LIB_PATH = ""
XML_PATH = ""
TEX_PATH = ""
DMI_PATH = ""
EXCLUDED = []

# Constants
VERSION = "0.8.0"
bl_info = {
    "name": "Journey Level Importer",
    "blender": (2, 80, 0),
    "category": "Import-Export",
    "description": "Import DecorationMeshInstances.lua from Journey",
}
xml_cache = {}
tex_cache = {}
errors = []
lib = None


@dataclass
class Entity(DataClassJsonMixin):
    objects: list[Object]
    transform: Transform
    armature: Optional[Armature] = None


@dataclass
class Object(DataClassJsonMixin):
    vertices: list[list[float]]
    uvs: list[list[float]]
    faces: list[list[int]]


@dataclass
class Armature(DataClassJsonMixin):
    root: Joint


@dataclass
class Joint(DataClassJsonMixin):
    name: str
    children: list[Joint]
    transform: Transform
    aliases: list[str]


@dataclass
class Transform(DataClassJsonMixin):
    matrix: list[list[float]]


@dataclass
class Options(DataClassJsonMixin):
    xml_root: str
    xml_file: str
    armature_full: bool = ARMATURE_FULL


def signature_check(lib_path, sig_path):
    import gnupg
    import requests  # type: ignore
    if not os.path.exists(lib_path) or not os.path.exists(sig_path):
        raise FileNotFoundError(f"Library '{lib_path}' or signature '{sig_path}' file not found!")

    gpg = gnupg.GPG()
    res = requests.get("https://github.com/arckoor.gpg")
    if res.status_code != 200:
        raise Exception("Failed to fetch public key!")
    p_key = res.text
    if gpg.import_keys(p_key).count != 1:
        raise Exception("Failed to import public key!")

    with open(sig_path, "rb") as file:
        verify = gpg.verify_file(file, lib_path, close_file=True)
        if not verify.valid or verify.fingerprint != "42F041970716D5F9A42468CE50E6472D9E5513E8":
            raise Exception("Invalid signature! YOU SHOULD UNDER NO CIRCUMSTANCES RUN THIS FILE!")


def load_lib():
    global lib
    system = platform.system()
    lib_path = LIB_PATH
    sig_path = LIB_PATH
    if system == "Linux":
        lib_path = os.path.join(lib_path, "libjourney_model_parser.so")
        sig_path = os.path.join(sig_path, "libjourney_model_parser.so.sig")
    elif system == "Windows":
        lib_path = os.path.join(lib_path, "journey_model_parser.dll")
        sig_path = os.path.join(sig_path, "journey_model_parser.dll.sig")

    if VERIFY_SIGNATURE:
        signature_check(lib_path, sig_path)
    else:
        pass
        raise Exception(
            "You are about to run an untrusted binary. Doing so can put your system at risk." +
            "\nRe-download the release artifacts from GitHub, and try again. Do not run binaries you received from any other source." +
            "\nIf you are sure you want to continue, comment out the raise statement the Blender python interpreter stopped at (in load_lib())."
            )

    lib = ctypes.CDLL(lib_path)

    lib.ffi_version.restype = ctypes.c_char_p
    lib.ffi_parse.restype = ctypes.POINTER(ctypes.c_char)
    lib.ffi_parse.argtypes = [ctypes.c_char_p]
    lib.ffi_free.argtypes = [ctypes.POINTER(ctypes.c_char)]

    version = lib.ffi_version().decode("utf-8")
    if version != VERSION:
        raise Exception(f"Version mismatch! Expected a binary with version {VERSION}, got {version}. Please update your binaries / this script.")


def create_cache(path, ext, cache):
    i = 0
    for root, _, files in os.walk(path):
        for file in files:
            if file.endswith(ext):
                filename_without_ext = os.path.splitext(file)[0]
                full_path = os.path.join(root, file)
                cache[filename_without_ext] = full_path
                i = i+1
    print(f"Cached {i} {ext} files.")


def find_from_cache(name, cache):
    for filename, full_path in cache.items():
        if name == filename:
            return full_path


def find_tex_from_tex_name(tex_name: str):
    if tex_name is None:
        return None
    return find_from_cache(tex_name.replace("P_", "")*2, tex_cache)


def traverse_lua_table(lua_table):
    count = len(lua_table)
    start = time.time()

    for i, (_, value) in enumerate(lua_table.items()):
        if isinstance(value, (lua_table.__class__,)):
            d = dict(value)
            mesh_name = d["Mesh"]
            if mesh_name.replace("P_", "") in EXCLUDED or mesh_name in EXCLUDED:
                continue

            matrix = None
            tex = None

            if isinstance(d["Transformation"], (lua_table.__class__,)):
                numbers = [c for _, r in dict(d["Transformation"]).items() for _, c in dict(r).items()]
                mat = [
                    [numbers[2], numbers[6], numbers[10], numbers[14]],
                    [numbers[0], numbers[4], numbers[8], numbers[12]],
                    [numbers[1], numbers[5], numbers[9], numbers[13]],
                    [numbers[3], numbers[7], numbers[11], numbers[15]]
                ]
                matrix = Matrix(mat)
            else:
                print(f"Failed to parse matrix for {mesh_name}")
                continue

            if isinstance(d["ShaderParams"], (lua_table.__class__,)):
                shader_params = dict(d["ShaderParams"])
                if not shader_params:
                    tex = "ClothAtlas"  # Change me to ClothAtlasGlow to get glowing cloth!
                else:
                    accepted_keys = ["texColor", "texCham", "tex", "textureAtlasA", "textureAtlasB"]
                    params = {
                        entry["ParamName"]: entry["ParamVal"]
                        for entry in d["ShaderParams"].values()
                        if entry["ParamName"] in accepted_keys and entry["ParamVal"] != "Blank"
                    }
                    tex = next((params[key] for key in accepted_keys if key in params), None)

            if not tex:
                print(f"Failed to parse texture for {mesh_name}")

            xml_name = find_from_cache(mesh_name, xml_cache)
            tex = find_tex_from_tex_name(tex)
            if xml_name is not None:
                print(f"({i+1}/{count}) Spawning {xml_name} for {mesh_name} with {tex}")
                spawn_models(xml_name, mesh_name, tex, matrix)
            else:
                print(f"Failed to find xml for {mesh_name}")

    print(f"Imported {count} models in {time.time() - start:.3f}s with {len(errors)} errors.")
    if errors:
        print("\nErrors:")
        for error in errors:
            print(error)


def spawn_models(xml_name, mesh_name, tex, transformation_matrix):
    options = Options(xml_root=XML_PATH, xml_file=f"{mesh_name}.xml")
    result = lib.ffi_parse(options.to_json().encode("utf-8"))

    if not result:
        errors.append(f"Failed to parse {xml_name}")
        print(f"Failed to parse {xml_name}")
        return

    try:
        data = ctypes.string_at(result).decode("utf-8")
        entity = Entity.from_json(data)
        objs = []
        translation_matrix = Matrix(entity.transform.matrix).transposed()
        for i, object in enumerate(entity.objects):
            print(f"Spawning model {i+1} of {len(entity.objects)} for {mesh_name}")
            obj = spawn_xml_model(object.vertices, object.uvs, object.faces, mesh_name, tex, transformation_matrix, translation_matrix, i > 0)
            objs.append(obj)
        if len(objs) > 1:
            for obj in objs:
                obj.select_set(True)
            bpy.context.view_layer.objects.active = objs[0]
            bpy.ops.object.join()
            bpy.ops.object.select_all(action="DESELECT")
        if entity.armature and len(objs) > 0:
            obj_name = objs[0].name
            armature_name = spawn_armature(obj_name, entity, transformation_matrix)
            obj = bpy.data.objects.get(obj_name)
            armature = bpy.data.objects.get(armature_name)
            obj.select_set(True)
            armature.select_set(True)
            bpy.context.view_layer.objects.active = armature
            bpy.ops.object.parent_set(type="ARMATURE")
            bpy.ops.object.mode_set(mode="OBJECT")
            bpy.ops.object.select_all(action="DESELECT")
    finally:
        lib.ffi_free(result)


def spawn_xml_model(vertices, uvs, faces, mesh_name, tex, transformation_matrix, translation_matrix, joined=False):
    if not vertices or not faces:
        errors.append(f"Invalid xml data for model:{mesh_name}! V:{len(vertices)} F:{len(faces)}")
        print(f"Invalid xml data for model:{mesh_name}! V:{len(vertices)} F:{len(faces)}")
        return

    mesh = bpy.data.meshes.new(name=mesh_name)
    mesh.from_pydata(vertices, [], faces)

    mesh.uv_layers.new(name="UVMap")
    uv_layer = mesh.uv_layers.active.data

    for face in mesh.polygons:
        for loop_idx in face.loop_indices:
            vert_idx = mesh.loops[loop_idx].vertex_index
            uv_layer[loop_idx].uv = uvs[vert_idx]

    mesh.transform(translation_matrix)
    mesh.update()

    obj = bpy.data.objects.new(mesh_name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.matrix_world = transformation_matrix

    if tex is not None and not joined:
        texture = bpy.data.images.load(tex)
        material = bpy.data.materials.new(name=f"{mesh_name}_material")
        material.use_nodes = True
        obj.data.materials.append(material)

        bsdf = material.node_tree.nodes.get("Principled BSDF")
        tex_image = material.node_tree.nodes.new("ShaderNodeTexImage")
        tex_image.image = texture

        material.node_tree.links.new(bsdf.inputs["Base Color"], tex_image.outputs["Color"])
        material.node_tree.links.new(bsdf.inputs["Alpha"], tex_image.outputs["Alpha"])

    return obj


def spawn_armature(object_name, entity: Entity, transformation_matrix):
    print(f"Spawning armature for {object_name}")
    bpy.ops.object.armature_add()
    armature = bpy.context.object
    armature.name = f"{object_name}_armature"
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="OBJECT")
    build_armature(armature, entity.armature.root)
    bpy.ops.object.mode_set(mode="EDIT")
    armature.data.edit_bones.remove(armature.data.edit_bones.get("Bone"))
    armature.matrix_world = transformation_matrix
    return armature.name


def build_armature(armature, node: Joint, parent_bone=None, parent_name=None, parent_matrix=None):
    combined_matrix = None
    matrix = Matrix(node.transform.matrix).transposed()
    if parent_matrix:
        if re.search(r"Flow(Hunter|Creature)_Head", armature.name):
            combined_matrix = matrix @ parent_matrix
        else:
            combined_matrix = parent_matrix @ matrix
    else:
        combined_matrix = matrix

    if parent_matrix and parent_matrix.to_translation() != combined_matrix.to_translation():
        bpy.context.view_layer.objects.active = armature
        bpy.context.view_layer.update()
        bpy.ops.object.mode_set(mode="EDIT")
        name = parent_name if parent_name != "root" else node.name
        bone = armature.data.edit_bones.new(name)
        if parent_bone:
            bone.parent = armature.data.edit_bones.get(parent_bone)
        bone.matrix = parent_matrix
        bone.tail = combined_matrix.to_translation()
        parent_bone = bone.name
        bpy.ops.object.mode_set(mode="OBJECT")

    parent_name = node.name

    for child in node.children:
        build_armature(armature, child, parent_bone, parent_name, combined_matrix)


if __name__ == "__main__":
    load_lib()
    create_cache(XML_PATH, ".xml", xml_cache)
    create_cache(TEX_PATH, ".png", tex_cache)

    os.chdir(DMI_PATH)
    lua = LuaRuntime(unpack_returned_tuples=True)
    lua.execute('dofile("DecorationMeshInstances.lua")')
    dmi_table = lua.globals().DecorationMeshInstances
    traverse_lua_table(dmi_table)
