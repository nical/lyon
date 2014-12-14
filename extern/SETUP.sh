#!/bin/sh

git submodule init
git submodule update

cd gl
make gen && make lib && make
cd ..

cd glfw
make
cd ..

cd png
./configure
make
cd ..

./BUILD.sh