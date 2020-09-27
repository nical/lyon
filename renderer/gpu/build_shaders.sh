#!/bin/sh
echo "build the shaders..."
glslc -DVERTEX_SHADER -fshader-stage=vertex ./shaders/quad.glsl -o ./shaders/quad.vert.spv &&
glslc -DFRAGMENT_SHADER -fshader-stage=fragment ./shaders/quad.glsl -o ./shaders/quad.frag.spv &&
glslc -DVERTEX_SHADER -fshader-stage=vertex ./shaders/mesh.glsl -o ./shaders/mesh.vert.spv &&
glslc -DFRAGMENT_SHADER -fshader-stage=fragment ./shaders/mesh.glsl -o ./shaders/mesh.frag.spv &&
echo "..done" ||
echoe "failed to build the shaders"

