#!/bin/sh
mkdir -p bin
RUST_BACKTRACE=1 rustc $* -g --out-dir bin src/vodk/scenegraph_test.rs -L src/vodk -L extern/lib -C link-args="-lglfw -lrt -lXrandr -lXi -lGL -lm -ldl -lXrender -ldrm -lXdamage -lX11-xcb -lxcb-glx -lxcb-dri2 -lXxf86vm -lXfixes -lXext -lX11 -lpthread -lxcb -lXau -lpng" && \
#RUST_BACKTRACE=1 rustc $* -g --out-dir bin src/vodk/cube_test.rs -L src/vodk -L extern/lib -C link-args="-lglfw -lrt -lXrandr -lXi -lGL -lm -ldl -lXrender -ldrm -lXdamage -lX11-xcb -lxcb-glx -lxcb-dri2 -lXxf86vm -lXfixes -lXext -lX11 -lpthread -lxcb -lXau -lpng" && \
#RUST_BACKTRACE=1 rustc $* -g --out-dir bin src/vodk/vertex_aa_test.rs -L src/vodk -L extern/lib -C link-args="-lglfw -lrt -lXrandr -lXi -lGL -lm -ldl -lXrender -ldrm -lXdamage -lX11-xcb -lxcb-glx -lxcb-dri2 -lXxf86vm -lXfixes -lXext -lX11 -lpthread -lxcb -lXau -lpng" && \
echo " -- done"