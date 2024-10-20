
from lupa import LuaRuntime
import os
import ctypes
import bpy  # type: ignore
from mathutils import Vector, Matrix  # type: ignore


LIB_PATH = ""
XML_PATH = ""
TEX_PATH = ""
DMI_PATH = ""

bl_info = {
    "name": "Journey Level Importer",
    "blender": (2, 80, 0),
    "category": "Import-Export",
    "description": "Import DecorationMeshInstances.lua from Journey",
}
xml_cache = {}
tex_cache = {}
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


def create_cache(path, ext, cache):
    i = 0
    for root, _, files in os.walk(path):
        for file in files:
            if file.endswith(ext):
                filename_without_ext = os.path.splitext(file)[0]
                full_path = os.path.join(root, file)
                cache[filename_without_ext] = full_path
                i = i+1
    print(f'Cached {i} {ext} files.')


def find_from_cache(name, cache):
    for filename, full_path in cache.items():
        if name in filename:
            return full_path


def find_tex_from_tex_name(tex_name: str):
    if tex_name is None:
        return None
    tex_name = tex_name.replace("P_", "")
    tex_name = f"{tex_name}{tex_name}"
    return find_from_cache(tex_name, tex_cache)


def traverse_lua_table(lua_table):
    for _, value in lua_table.items():
        if isinstance(value, (lua_table.__class__,)):
            d = dict(value)
            mat = None
            tex = None

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

            if isinstance(d["ShaderParams"], (lua_table.__class__,)):
                for _, entry in d["ShaderParams"].items():
                    if entry["ParamName"] == "texColor":
                        tex_ = entry["ParamVal"]
                        if tex == "Blank":
                            continue
                        tex = tex_
                        break
                    elif entry["ParamName"] == "texCham":
                        tex = entry["ParamVal"]

            if not tex:
                print("Failed to parse texture")

            mesh_name = d["Mesh"]
            matrix = Matrix(mat)

            xml_name = find_from_cache(mesh_name, xml_cache)
            tex = find_tex_from_tex_name(tex)
            if xml_name is not None:
                print(f"Spawning {xml_name} for {mesh_name} with {tex}")
                spawn_xml_model(xml_name, mesh_name, tex, matrix)


def spawn_xml_model(xml_file, mesh_name, tex, transformation_matrix):
    result = lib.ffi_parse(xml_file.encode("utf-8"))

    if not result:
        print(f"Failed to parse {xml_file}")
        return

    result = result.contents

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

    if tex is not None:
        texture = bpy.data.images.load(tex)
        material = bpy.data.materials.new(name=f"{mesh_name}_material")
        material.use_nodes = True
        obj.data.materials.append(material)

        bsdf = material.node_tree.nodes.get('Principled BSDF')
        tex_image = material.node_tree.nodes.new('ShaderNodeTexImage')
        tex_image.image = texture

        material.node_tree.links.new(bsdf.inputs['Base Color'], tex_image.outputs['Color'])


if __name__ == "__main__":
    create_cache(XML_PATH, '.xml', xml_cache)
    create_cache(TEX_PATH, '.png', tex_cache)

    os.chdir(DMI_PATH)
    lua = LuaRuntime(unpack_returned_tuples=True)
    lua.execute('dofile("DecorationMeshInstances.lua")')
    dmi_table = lua.globals().DecorationMeshInstances
    traverse_lua_table(dmi_table)
