// C ABI shim over OpenCASCADE (OCCT).
//
// Every OCCT type stays on this side of the boundary: shapes are handed to
// Rust as opaque `void*` handles owned by `TopoDS_Shape`.
//
// Compiled only when the `occt` feature is enabled (see build.rs).

#include <cstring>
#include <new>
#include <utility>
#include <vector>

#include <IFSelect_RetDone.hxx>
#include <Standard_Version.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_StepModelType.hxx>
#include <STEPControl_Writer.hxx>
#include <TopAbs.hxx>
#include <TopExp_Explorer.hxx>
#include <TopLoc_Location.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>

#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRep_Tool.hxx>
#include <Poly_Triangulation.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>

namespace {

void* wrap(const TopoDS_Shape& shape) {
    if (shape.IsNull()) {
        return nullptr;
    }
    return new (std::nothrow) TopoDS_Shape(shape);
}

const TopoDS_Shape* as_shape(void* handle) {
    return static_cast<const TopoDS_Shape*>(handle);
}

TopoDS_Shape* as_mutable_shape(void* handle) {
    return static_cast<TopoDS_Shape*>(handle);
}

}  // namespace

extern "C" {

int pf_occt_init(void) {
    try {
        // Touch a simple primitive to force the TKernel/TKPrim libraries in.
        BRepPrimAPI_MakeBox probe(1.0, 1.0, 1.0);
        return probe.IsDone() ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

const char* pf_occt_version(void) {
#ifdef OCC_VERSION_COMPLETE
    return OCC_VERSION_COMPLETE;
#else
    return "unknown";
#endif
}

void* pf_box(double dx, double dy, double dz) {
    try {
        return wrap(BRepPrimAPI_MakeBox(dx, dy, dz).Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_cylinder(double radius, double height) {
    try {
        return wrap(BRepPrimAPI_MakeCylinder(radius, height).Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_sphere(double radius) {
    try {
        return wrap(BRepPrimAPI_MakeSphere(radius).Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_cone(double r1, double r2, double height) {
    try {
        return wrap(BRepPrimAPI_MakeCone(r1, r2, height).Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_fuse(void* a, void* b) {
    if (!a || !b) {
        return nullptr;
    }
    try {
        BRepAlgoAPI_Fuse op(*as_shape(a), *as_shape(b));
        op.Build();
        if (!op.IsDone()) {
            return nullptr;
        }
        return wrap(op.Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_cut(void* a, void* b) {
    if (!a || !b) {
        return nullptr;
    }
    try {
        BRepAlgoAPI_Cut op(*as_shape(a), *as_shape(b));
        op.Build();
        if (!op.IsDone()) {
            return nullptr;
        }
        return wrap(op.Shape());
    } catch (...) {
        return nullptr;
    }
}

void* pf_fillet(void* shape, double radius) {
    if (!shape || radius <= 0.0) {
        return nullptr;
    }
    try {
        const TopoDS_Shape& base = *as_shape(shape);
        BRepFilletAPI_MakeFillet fillet(base);
        for (TopExp_Explorer ex(base, TopAbs_EDGE); ex.More(); ex.Next()) {
            fillet.Add(radius, TopoDS::Edge(ex.Current()));
        }
        fillet.Build();
        if (!fillet.IsDone()) {
            return nullptr;
        }
        return wrap(fillet.Shape());
    } catch (...) {
        return nullptr;
    }
}

void pf_shape_free(void* shape) {
    delete as_mutable_shape(shape);
}

int pf_triangulate(void* shape, double deflection, double** out_positions,
                   unsigned int* out_triangle_count) {
    if (!shape || !out_positions || !out_triangle_count) {
        return 0;
    }
    *out_positions = nullptr;
    *out_triangle_count = 0;

    try {
        const TopoDS_Shape& base = *as_shape(shape);
        const double linear = deflection > 0.0 ? deflection : 0.1;
        BRepMesh_IncrementalMesh mesher(base, linear, Standard_False, 0.5,
                                        Standard_True);
        mesher.Perform();

        std::vector<double> vertices;
        for (TopExp_Explorer ex(base, TopAbs_FACE); ex.More(); ex.Next()) {
            const TopoDS_Face face = TopoDS::Face(ex.Current());
            TopLoc_Location location;
            const Handle(Poly_Triangulation)& tri =
                BRep_Tool::Triangulation(face, location);
            if (tri.IsNull()) {
                continue;
            }
            const gp_Trsf trsf = location.Transformation();
            const bool reversed = (face.Orientation() == TopAbs_REVERSED);

            for (Standard_Integer i = 1; i <= tri->NbTriangles(); ++i) {
                Standard_Integer n1 = 0;
                Standard_Integer n2 = 0;
                Standard_Integer n3 = 0;
                tri->Triangle(i).Get(n1, n2, n3);
                if (reversed) {
                    std::swap(n2, n3);
                }
                const Standard_Integer nodes[3] = {n1, n2, n3};
                for (int k = 0; k < 3; ++k) {
                    const gp_Pnt p = tri->Node(nodes[k]).Transformed(trsf);
                    vertices.push_back(p.X());
                    vertices.push_back(p.Y());
                    vertices.push_back(p.Z());
                }
            }
        }

        if (vertices.empty()) {
            return 0;
        }
        const std::size_t count = vertices.size();
        double* buffer = new (std::nothrow) double[count];
        if (!buffer) {
            return 0;
        }
        std::memcpy(buffer, vertices.data(), count * sizeof(double));
        *out_positions = buffer;
        *out_triangle_count = static_cast<unsigned int>(count / 9);
        return 1;
    } catch (...) {
        return 0;
    }
}

void pf_positions_free(double* positions) {
    delete[] positions;
}

int pf_step_read(const char* path, void** out) {
    if (!path || !out) {
        return 0;
    }
    *out = nullptr;
    try {
        STEPControl_Reader reader;
        if (reader.ReadFile(path) != IFSelect_RetDone) {
            return 0;
        }
        reader.TransferRoots();
        const TopoDS_Shape shape = reader.OneShape();
        if (shape.IsNull()) {
            return 0;
        }
        *out = wrap(shape);
        return *out ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

int pf_step_write(void* shape, const char* path) {
    if (!shape || !path) {
        return 0;
    }
    try {
        STEPControl_Writer writer;
        if (writer.Transfer(*as_shape(shape), STEPControl_AsIs) !=
            IFSelect_RetDone) {
            return 0;
        }
        return writer.Write(path) == IFSelect_RetDone ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

}  // extern "C"
