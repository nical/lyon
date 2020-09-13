#!/bin/sh
glslc -DVERTEX_SHADER -fshader-stage=vertex ./shaders/quad.glsl -o ./shaders/quad.vert.spv
glslc -DFRAGMENT_SHADER -fshader-stage=fragment ./shaders/quad.glsl -o ./shaders/quad.frag.spv

