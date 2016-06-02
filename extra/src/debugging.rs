use path::{ Path, PathSlice };
use path_builder::{ flattened_path_builder, PrimitiveBuilder };
use vodk_math::{ Vec2 };

pub type Polygons = Vec<Vec<Vec2>>;

pub fn path_to_polygons(path: PathSlice) -> Vec<Vec<Vec2>> {
    let mut polygons = Vec::new();
    for sub in path.path_ids() {
        let sub_path = path.sub_path(sub);
        let mut poly = Vec::new();
        for it in sub_path.vertex_ids() {
            poly.push(sub_path.vertex(it).position);
        }
        polygons.push(poly);
    }
    return polygons;
}

pub fn polygons_to_path(polygons: &Polygons) -> Path {
    let mut builder = flattened_path_builder();
    for poly in polygons.iter() {
        builder.move_to(poly[0]);
        for i in 1..poly.len() {
            builder.line_to(poly[i]);
        }
        builder.close();
    }
    return builder.build();
}

pub fn find_reduced_test_case<F: Fn(Path)->bool+panic::UnwindSafe+panic::RefUnwindSafe>(path: PathSlice, cb: &F) -> Path {
    let mut polygons = path_to_polygons(path);

    println!(" -- removing sub-paths...");

    polygons = find_reduced_test_case_sp(polygons, cb);

    println!(" -- removing vertices...");

    for p in 0..polygons.len() {
        let mut v = 0;
        loop {
            if v >= polygons[p].len() || polygons[p].len() <= 3 {
                break;
            }

            let mut cloned = polygons.clone();
            cloned[p].remove(v);

            let path = polygons_to_path(&cloned);

            let failed = panic::catch_unwind(|| { cb(path) }).unwrap_or(true);

            if failed {
                polygons = cloned;
                continue;
            }

            v +=1 ;
        }
    }

    println!(" ----------- reduced test case: -----------");
    for p in 0..polygons.len() {
        let pos = polygons[p][0];
        println!("builder.move_to(vec2({}, {}));", pos.x, pos.y);
        for v in 1..polygons[p].len() {
            let pos = polygons[p][v];
            println!("builder.line_to(vec2({}, {}));", pos.x, pos.y);
        }
        println!("builder.close();");
    }

    return polygons_to_path(&polygons);
}

use std::panic;

fn find_reduced_test_case_sp<F: Fn(Path)->bool+panic::UnwindSafe+panic::RefUnwindSafe>(mut polygons: Polygons, cb: &F) -> Polygons {
    let mut i = 0;
    loop {
        if i >= polygons.len() {
            return polygons;
        }

        let mut cloned = polygons.clone();
        cloned.remove(i);
        let path = polygons_to_path(&cloned);

        let failed = panic::catch_unwind(|| { cb(path) }).unwrap_or(true);

        if failed {
            polygons = cloned;
            continue;
        }

        i += 1;
    }
}

