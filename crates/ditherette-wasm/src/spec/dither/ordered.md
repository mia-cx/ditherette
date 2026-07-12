# dither algorithm spec

Executable oracle for this dithering family. Inputs are precomputed f32 working-space coordinates plus palette coordinates in the same space; outputs are `PaletteIndex8` entries with stable palette-order tie behavior inherited from nearest matching.
