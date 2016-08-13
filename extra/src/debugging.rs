use lyon_path::{ Path, Path2, PathSlice, PathSlice2, flattened_path_builder, flattened_path_builder2 };
use lyon_core::{ PrimitiveEvent };
use lyon_path_builder::PrimitiveBuilder;
use lyon_path_iterator::PositionedPrimitiveIter;
use math::{ Vec2 };

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

pub fn path_to_polygons2(path: PathSlice2) -> Vec<Vec<Vec2>> {
    let mut polygons = Vec::new();
    let mut poly = Vec::new();
    for evt in PositionedPrimitiveIter::new(path.iter()) {
        match evt {
            PrimitiveEvent::MoveTo(to) => {
                polygons.push(poly);
                poly = vec![to];
            }
            PrimitiveEvent::LineTo(to) => {
                poly.push(to);
            }
            PrimitiveEvent::Close => {
                if !poly.is_empty() {
                    polygons.push(poly);
                }
                poly = Vec::new();
            }
            _ => {
                println!(" -- path_to_polygons: warning! Unsupported event type {:?}", evt);
            }
        }
    }
    return polygons;
}

pub fn polygons_to_path(polygons: &Polygons) -> Path {
    let mut builder = flattened_path_builder(0.05);
    for poly in polygons.iter() {
        builder.move_to(poly[0]);
        for i in 1..poly.len() {
            builder.line_to(poly[i]);
        }
        builder.close();
    }
    return builder.build();
}

pub fn polygons_to_path2(polygons: &Polygons) -> Path2 {
    let mut builder = flattened_path_builder2(0.05);
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

    println!(" ----------- reduced test case: -----------\n\n");
    println!("#[test]");
    println!("fn reduced_test_case() {{");
    println!("    let mut builder = flattened_path_builder(0.05);\n");
    for p in 0..polygons.len() {
        let pos = polygons[p][0];
        println!("    builder.move_to(vec2({}, {}));", pos.x, pos.y);
        for v in 1..polygons[p].len() {
            let pos = polygons[p][v];
            println!("    builder.line_to(vec2({}, {}));", pos.x, pos.y);
        }
        println!("    builder.close();\n");
    }
    println!("    test_path(builder.build().as_slice(), None);");
    println!("}}\n\n");

    return polygons_to_path(&polygons);
}

pub fn find_reduced_test_case2<F: Fn(Path2)->bool+panic::UnwindSafe+panic::RefUnwindSafe>(path: PathSlice2, cb: &F) -> Path2 {
    let mut polygons = path_to_polygons2(path);

    println!(" -- removing sub-paths...");

    polygons = find_reduced_test_case_sp2(polygons, cb);

    println!(" -- removing vertices...");

    for p in 0..polygons.len() {
        let mut v = 0;
        loop {
            if v >= polygons[p].len() || polygons[p].len() <= 3 {
                break;
            }

            let mut cloned = polygons.clone();
            cloned[p].remove(v);

            let path = polygons_to_path2(&cloned);

            let failed = panic::catch_unwind(|| { cb(path) }).unwrap_or(true);

            if failed {
                polygons = cloned;
                continue;
            }

            v +=1 ;
        }
    }

    println!(" ----------- reduced test case: -----------\n\n");
    println!("#[test]");
    println!("fn reduced_test_case() {{");
    println!("    let mut builder = flattened_path_builder(0.05);\n");
    for p in 0..polygons.len() {
        let pos = polygons[p][0];
        println!("    builder.move_to(vec2({}, {}));", pos.x, pos.y);
        for v in 1..polygons[p].len() {
            let pos = polygons[p][v];
            println!("    builder.line_to(vec2({}, {}));", pos.x, pos.y);
        }
        println!("    builder.close();\n");
    }
    println!("    test_path(builder.build().as_slice(), None);");
    println!("}}\n\n");

    return polygons_to_path2(&polygons);
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

fn find_reduced_test_case_sp2<F: Fn(Path2)->bool+panic::UnwindSafe+panic::RefUnwindSafe>(mut polygons: Polygons, cb: &F) -> Polygons {
    let mut i = 0;
    loop {
        if i >= polygons.len() {
            return polygons;
        }

        let mut cloned = polygons.clone();
        cloned.remove(i);
        let path = polygons_to_path2(&cloned);

        let failed = panic::catch_unwind(|| { cb(path) }).unwrap_or(true);

        if failed {
            polygons = cloned;
            continue;
        }

        i += 1;
    }
}

#[test]
fn test_path_to_polygons() {
    // TODO!
}