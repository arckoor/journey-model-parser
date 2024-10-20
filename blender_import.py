
from lupa import LuaRuntime
import os
import ctypes
import bpy  # type: ignore
from mathutils import Vector, Matrix  # type: ignore


LIB_PATH = ""
XML_PATH = ""
DMI_PATH = ""

bl_info = {
    "name": "Journey Level Importer",
    "blender": (2, 80, 0),
    "category": "Import-Export",
    "description": "Import DecorationMeshInstances.lua from Journey",
}
xml_cache = {}
lib = ctypes.CDLL(LIB_PATH)


class ParsedModelData(ctypes.Structure):
    _fields_ = [
        ("vertices_ptr", ctypes.POINTER(ctypes.c_float)),
        ("vertices_len", ctypes.c_size_t),
        ("uvs_ptr", ctypes.POINTER(ctypes.c_float)),
        ("uvs_len", ctypes.c_size_t),
        ("faces_ptr", ctypes.POINTER(ctypes.c_ulonglong)),
        ("faces_len", ctypes.c_size_t),
    ]


lib.ffi_parse.restype = ctypes.POINTER(ParsedModelData)
lib.ffi_parse.argtypes = [ctypes.c_char_p]
lib.ffi_free.argtypes = [ctypes.POINTER(ParsedModelData)]


def cache_xml():
    i = 0
    for root, _, files in os.walk(XML_PATH):
        for file in files:
            if file.endswith('.xml'):
                filename_without_ext = os.path.splitext(file)[0]
                full_path = os.path.join(root, file)
                xml_cache[filename_without_ext] = full_path
                i = i+1
    print(f'Cached {i} xml files.')


def find_xml_from_mesh_name(mesh_name):
    for filename, full_path in xml_cache.items():
        if mesh_name in filename:
            return full_path


def traverse_lua_table(lua_table):
    for _, value in lua_table.items():
        if isinstance(value, (lua_table.__class__,)):
            d = dict(value)
            mat = None

            if isinstance(d["Transformation"], (lua_table.__class__,)):
                numbers = [c for _, r in dict(d["Transformation"]).items() for _, c in dict(r).items()]
                mat = [
                    [numbers[2], numbers[6], numbers[10], numbers[14]],
                    [numbers[0], numbers[4], numbers[8], numbers[12]],
                    [numbers[1], numbers[5], numbers[9], numbers[13]],
                    [numbers[3], numbers[7], numbers[11], numbers[15]]
                ]
            if not mat:
                print("Failed to parse matrix")
                continue

            mesh_name = d["Mesh"]
            matrix = Matrix(mat)

            xml_name = find_xml_from_mesh_name(mesh_name)
            if xml_name is not None:
                print(f"Spawning {xml_name} for {mesh_name}")
                spawn_xml_model(xml_name, mesh_name, matrix)


def spawn_xml_model(xml_file, mesh_name, transformation_matrix):
    result = lib.ffi_parse(xml_file.encode("utf-8")).contents

    if result is None:
        print(f"Failed to parse {xml_file}")
        return

    vertices_flat = [round(result.vertices_ptr[i], 4) for i in range(result.vertices_len)]
    vertices = [Vector((vertices_flat[i], vertices_flat[i + 1], vertices_flat[i + 2])) for i in range(0, len(vertices_flat), 3)]

    uvs_flat = [round(result.uvs_ptr[i], 4) for i in range(result.uvs_len)]
    uvs = [(uvs_flat[i], uvs_flat[i + 1]) for i in range(0, len(uvs_flat), 2)]

    faces_flat = [round(result.faces_ptr[i], 4) for i in range(result.faces_len)]
    faces = [(faces_flat[i], faces_flat[i + 1], faces_flat[i + 2]) for i in range(0, len(faces_flat), 3)]

    lib.ffi_free(result)

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


if __name__ == "__main__":
    cache_xml()

    os.chdir(DMI_PATH)
    lua = LuaRuntime(unpack_returned_tuples=True)
    lua.execute('dofile("DecorationMeshInstances.lua")')
    dmi_table = lua.globals().DecorationMeshInstances
    traverse_lua_table(dmi_table)
