{application, prism_beam, [
    {vsn, "0.1.0"},
    {applications, [gleam_stdlib,
                    gleeunit]},
    {description, "Prism types for the BEAM — Beam(t), Oid, ShannonLoss, Precision, Pressure"},
    {modules, [prism_beam,
               prism_beam@@main,
               prism_beam_test]},
    {registered, []}
]}.
