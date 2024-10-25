
import ctypes
import os
import platform
import requests

import bpy  # type: ignore
from lupa import LuaRuntime
from mathutils import Vector, Matrix  # type: ignore

# Configuration
VERIFY_SIGNATURE = True  # Verify the library signature. DISABLING THIS IS NOT RECOMMENDED!
LIB_PATH = ""
XML_PATH = ""
TEX_PATH = ""
DMI_PATH = ""
EXCLUDED = []

# Constants
VERSION = "0.6.0"
bl_info = {
    "name": "Journey Level Importer",
    "blender": (2, 80, 0),
    "category": "Import-Export",
    "description": "Import DecorationMeshInstances.lua from Journey",
}
xml_cache = {}
tex_cache = {}
lib = None


class ParsedModelData(ctypes.Structure):
    _fields_ = [
        ("object_count", ctypes.c_size_t),
        ("vertices_ptr", ctypes.POINTER(ctypes.c_float)),
        ("uvs_ptr", ctypes.POINTER(ctypes.c_float)),
        ("faces_ptr", ctypes.POINTER(ctypes.c_ulonglong)),
        ("vertices_len_ptr", ctypes.POINTER(ctypes.c_size_t)),
        ("uvs_len_ptr", ctypes.POINTER(ctypes.c_size_t)),
        ("faces_len_ptr", ctypes.POINTER(ctypes.c_size_t)),
    ]


def signature_check(lib_path, sig_path):
    import gnupg
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
    lib.ffi_parse.restype = ctypes.POINTER(ParsedModelData)
    lib.ffi_parse.argtypes = [ctypes.c_char_p]
    lib.ffi_free.argtypes = [ctypes.POINTER(ParsedModelData)]

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
    for _, value in lua_table.items():
        if isinstance(value, (lua_table.__class__,)):
            d = dict(value)
            mesh_name = d["Mesh"]
            if mesh_name.replace("P_", "") in EXCLUDED:
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
                print("Failed to parse matrix")
                continue

            if isinstance(d["ShaderParams"], (lua_table.__class__,)):
                shader_params = dict(d["ShaderParams"])
                if not shader_params:
                    tex = "ClothAtlas"
                else:
                    accepted_keys = ["texColor", "texCham", "tex"]
                    params = {
                        entry["ParamName"]: entry["ParamVal"]
                        for entry in d["ShaderParams"].values()
                        if entry["ParamName"] in accepted_keys
                    }
                    tex = next((params[key] for key in accepted_keys if key in params), None)

            if not tex:
                print("Failed to parse texture")

            xml_name = find_from_cache(mesh_name, xml_cache)
            tex = find_tex_from_tex_name(tex)
            if xml_name is not None:
                print(f"Spawning {xml_name} for {mesh_name} with {tex}")
                spawn_models(xml_name, mesh_name, tex, matrix)


def spawn_models(xml_name, mesh_name, tex, transformation_matrix):
    result = lib.ffi_parse(xml_name.encode("utf-8"))

    if not result:
        print(f"Failed to parse {xml_name}")
        return

    result = result.contents
    try:
        for i in range(result.object_count):
            vertices_start = sum(result.vertices_len_ptr[:i])
            uvs_start = sum(result.uvs_len_ptr[:i])
            faces_start = sum(result.faces_len_ptr[:i])

            vertices_flat = [round(result.vertices_ptr[i], 4) for i in range(vertices_start, vertices_start + result.vertices_len_ptr[i])]
            uvs_flat = [round(result.uvs_ptr[i], 4) for i in range(uvs_start, uvs_start + result.uvs_len_ptr[i])]
            faces_flat = [result.faces_ptr[i] for i in range(faces_start, faces_start + result.faces_len_ptr[i])]

            vertices = [Vector(vertices_flat[i:i+3]) for i in range(0, len(vertices_flat), 3)]
            uvs = [uvs_flat[i:i+2] for i in range(0, len(uvs_flat), 2)]
            faces = [faces_flat[i:i+3] for i in range(0, len(faces_flat), 3)]

            print(f"Spawning model {i+1} of {result.object_count} for {mesh_name}")
            spawn_xml_model(vertices, uvs, faces, mesh_name, tex, transformation_matrix)
    finally:
        lib.ffi_free(result)


def spawn_xml_model(vertices, uvs, faces, mesh_name, tex, transformation_matrix):
    if not vertices or not faces:
        print(f"Invalid xml data for model:{mesh_name}! V:{len(vertices)} F:{len(faces)}")
        return

    mesh = bpy.data.meshes.new(name=mesh_name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()

    mesh.uv_layers.new(name="UVMap")
    uv_layer = mesh.uv_layers.active.data

    for face in mesh.polygons:
        for loop_idx in face.loop_indices:
            vert_idx = mesh.loops[loop_idx].vertex_index
            uv_layer[loop_idx].uv = uvs[vert_idx]

    mesh.update()

    obj = bpy.data.objects.new(mesh_name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.matrix_world = transformation_matrix

    if tex is not None:
        texture = bpy.data.images.load(tex)
        material = bpy.data.materials.new(name=f"{mesh_name}_material")
        material.use_nodes = True
        obj.data.materials.append(material)

        bsdf = material.node_tree.nodes.get("Principled BSDF")
        tex_image = material.node_tree.nodes.new("ShaderNodeTexImage")
        tex_image.image = texture

        material.node_tree.links.new(bsdf.inputs["Base Color"], tex_image.outputs["Color"])
        material.node_tree.links.new(bsdf.inputs["Alpha"], tex_image.outputs["Alpha"])


if __name__ == "__main__":
    load_lib()
    create_cache(XML_PATH, ".xml", xml_cache)
    create_cache(TEX_PATH, ".png", tex_cache)

    os.chdir(DMI_PATH)
    lua = LuaRuntime(unpack_returned_tuples=True)
    lua.execute('dofile("DecorationMeshInstances.lua")')
    dmi_table = lua.globals().DecorationMeshInstances
    traverse_lua_table(dmi_table)
