use crate::geometry_builder::{VertexBuffers, simple_builder};
use crate::path::{Path, PathSlice};
use crate::path_fill::*;
use crate::geom::math::*;
use crate::FillVertex;
use crate::FillOptions;
use crate::TessellationError;
use crate::OnError;

#[cfg(feature = "experimental")]
use crate::experimental;

#[cfg(not(feature = "experimental"))]
type Vertex = FillVertex;
#[cfg(feature = "experimental")]
type Vertex = Point;

fn tessellate_path(path: PathSlice, log: bool, on_error: OnError) -> Result<usize, TessellationError> {
    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    {
        let options = FillOptions::tolerance(0.05).on_error(on_error);

        #[cfg(not(feature = "experimental"))] {
            let mut tess = FillTessellator::new();
            let mut vertex_builder = simple_builder(&mut buffers);
            if log {
                tess.enable_logging();
            }
            tess.tessellate_path(
                path.iter(),
                &options,
                &mut vertex_builder
            )?;
        }

        #[cfg(feature = "experimental")] {
            use crate::path::builder::*;
            use crate::path::iterator::*;

            let mut builder = Path::builder();
            for e in path.iter().flattened(0.05) {
                builder.path_event(e);
            }

            let mut vertex_builder = simple_builder(&mut buffers);
            let mut tess = experimental::FillTessellator::new();
            if log {
                tess.enable_logging();
            }
            tess.tessellate_path(
                &builder.build(),
                &options,
                &mut vertex_builder
            );
        }
    }
    return Ok(buffers.indices.len() / 3);
}

fn test_path(path: PathSlice) {
    let add_logging = std::env::var("LYON_ENABLE_LOGGING").is_ok();
    let find_test_case = std::env::var("LYON_REDUCED_TESTCASE").is_ok();

    let on_error = OnError::Panic;
    let res = ::std::panic::catch_unwind(|| tessellate_path(path, false, on_error));

    if res.is_ok() {
        return;
    }

    // First see if the tessellator detect the error without panicking
    let recover_mode = ::std::panic::catch_unwind(
        || tessellate_path(path, false, OnError::Recover)
    );

    if find_test_case {
        crate::extra::debugging::find_reduced_test_case(
            path,
            &|path: Path| { return tessellate_path(path.as_slice(), false, on_error).is_err(); },
        );
    }

    print!(" -- Tessellating with OnError::Recover ");
    if let Ok(result) = recover_mode {
        println!("returned {:?}.", result);
    } else {
        println!("panicked.");
    }

    if add_logging {
        tessellate_path(path, true, on_error).unwrap();
    }

    panic!("Test failed.");
}

#[test]
fn fuzzing_test_case_01() {
    let mut builder = Path::builder();

    builder.move_to(point(0.78730774, 0.48590088));
    builder.line_to(point(0.9696454, 0.6628016));
    builder.line_to(point(0.34390104, 0.16192567));
    builder.line_to(point(0.6777611, 0.27082264));
    builder.line_to(point(0.56993425, 0.36398673));
    builder.line_to(point(0.7553669, 0.8379742));
    builder.line_to(point(0.10334098, 0.2124151));
    builder.line_to(point(0.058819532, 0.25938368));
    builder.line_to(point(0.4545982, 0.7907194));
    builder.line_to(point(0.11562657, 0.98054576));
    builder.line_to(point(0.58857, 0.35739875));
    builder.line_to(point(0.7018006, 0.48710144));
    builder.line_to(point(0.32512426, 0.14753413));
    builder.line_to(point(0.29062843, 0.86347556));
    builder.line_to(point(0.163795, 0.044541));
    builder.line_to(point(0.64731395, 0.06582558));
    builder.line_to(point(0.3953712, 0.7253332));
    builder.line_to(point(0.5990387, 0.0978142));
    builder.line_to(point(0.51700723, 0.29514837));
    builder.line_to(point(0.37555957, 0.36456883));
    builder.line_to(point(0.022779346, 0.041197658));
    builder.line_to(point(0.98860896, 0.9846177));
    builder.line_to(point(0.38955593, 0.23815441));
    builder.line_to(point(0.12912107, 0.8679553));
    builder.line_to(point(0.20826244, 0.08163428));
    builder.line_to(point(0.8907114, 0.13253903));
    builder.line_to(point(0.49465072, 0.5307982));
    builder.line_to(point(0.43128633, 0.4072647));
    builder.line_to(point(0.6622015, 0.025433421));
    builder.line_to(point(0.17607379, 0.7340293));
    builder.line_to(point(0.89449656, 0.4222151));
    builder.line_to(point(0.33659184, 0.10005617));
    builder.line_to(point(0.8160367, 0.7902672));
    builder.line_to(point(0.3394419, 0.26354945));
    builder.line_to(point(0.74704313, 0.3669362));
    builder.line_to(point(0.882795, 0.24774492));
    builder.line_to(point(0.22983181, 0.35437965));
    builder.line_to(point(0.61653507, 0.5209358));
    builder.line_to(point(0.07520425, 0.3861009));
    builder.line_to(point(0.6261513, 0.3076942));
    builder.line_to(point(0.89616644, 0.14718497));
    builder.line_to(point(0.15250742, 0.33876193));
    builder.line_to(point(0.96403444, 0.73444545));
    builder.line_to(point(0.7839006, 0.30109072));
    builder.line_to(point(0.1244781, 0.287135));
    builder.line_to(point(0.7767385, 0.13594544));
    builder.line_to(point(0.454705, 0.14277875));
    builder.line_to(point(0.9495021, 0.5886166));
    builder.line_to(point(0.24866652, 0.28273904));
    builder.line_to(point(0.672814, 0.4579798));
    builder.line_to(point(0.27975917, 0.19149947));
    builder.line_to(point(0.56860876, 0.883263));
    builder.line_to(point(0.75454605, 0.58421946));
    builder.line_to(point(0.86330116, 0.5277505));
    builder.line_to(point(0.47075093, 0.18962681));
    builder.line_to(point(0.264279, 0.15683436));
    builder.line_to(point(0.68764293, 0.88234806));
    builder.line_to(point(0.42361128, 0.54266036));
    builder.line_to(point(0.7556609, 0.1417911));
    builder.line_to(point(0.88452077, 0.7777879));
    builder.line_to(point(0.8501849, 0.92232525));
    builder.line_to(point(0.45093215, 0.58600414));
    builder.line_to(point(0.7537575, 0.57182527));
    builder.line_to(point(0.31972456, 0.34851098));
    builder.line_to(point(0.23725474, 0.051594973));
    builder.line_to(point(0.44865406, 0.83957255));
    builder.line_to(point(0.58956456, 0.06745672));
    builder.line_to(point(0.17060673, 0.35480642));
    builder.line_to(point(0.28965175, 0.6841849));
    builder.line_to(point(0.24731481, 0.5745305));
    builder.line_to(point(0.0026792288, 0.18591964));
    builder.line_to(point(0.2517339, 0.5779605));
    builder.line_to(point(0.38850832, 0.9764265));
    builder.line_to(point(0.37909698, 0.03419876));
    builder.line_to(point(0.3823061, 0.5899316));
    builder.line_to(point(0.3344624, 0.5034019));
    builder.line_to(point(0.34625828, 0.29476762));
    builder.line_to(point(0.7707838, 0.85849));
    builder.line_to(point(0.1608665, 0.005480051));
    builder.line_to(point(0.41175807, 0.8414284));
    builder.line_to(point(0.11086798, 0.027983546));
    builder.line_to(point(0.42707598, 0.03993404));
    builder.line_to(point(0.5653765, 0.5821123));
    builder.line_to(point(0.935071, 0.60360384));
    builder.line_to(point(0.3218763, 0.9014677));
    builder.line_to(point(0.570966, 0.17866242));
    builder.line_to(point(0.7075348, 0.8523464));
    builder.line_to(point(0.5388646, 0.35146892));
    builder.line_to(point(0.44184422, 0.09739721));
    builder.line_to(point(0.19552732, 0.8780161));
    builder.line_to(point(0.028696775, 0.6640192));
    builder.line_to(point(0.73951757, 0.3810749));
    builder.line_to(point(0.4420668, 0.05925262));
    builder.line_to(point(0.54023945, 0.16737175));
    builder.line_to(point(0.8839954, 0.39966547));
    builder.line_to(point(0.5651517, 0.5119977));
    builder.line_to(point(0.10021269, 0.17348659));
    builder.line_to(point(0.77066493, 0.67880523));
    builder.line_to(point(0.90599084, 0.07677424));
    builder.line_to(point(0.8447012, 0.84064853));
    builder.line_to(point(0.48341691, 0.09270251));
    builder.line_to(point(0.053493023, 0.18919432));
    builder.line_to(point(0.41371357, 0.76999104));
    builder.line_to(point(0.52973855, 0.59926045));
    builder.line_to(point(0.6088793, 0.57187665));
    builder.line_to(point(0.2899257, 0.09821439));
    builder.line_to(point(0.1324873, 0.9954816));
    builder.line_to(point(0.31996012, 0.2892481));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_2() {
    let mut builder = Path::builder();

    builder.move_to(point(97.84245, 342.3357));
    builder.line_to(point(827.032, 869.6488));
    builder.line_to(point(100.65568, 711.0392));
    builder.line_to(point(160.9056, 325.1753));
    builder.line_to(point(734.98535, 558.55383));
    builder.line_to(point(345.89362, 475.03342));
    builder.line_to(point(435.53317, 729.7931));
    builder.line_to(point(623.16095, 737.8609));
    builder.line_to(point(674.4163, 13.610005));
    builder.line_to(point(602.3282, 853.93646));
    builder.line_to(point(300.64703, 553.5566));
    builder.line_to(point(675.2951, 383.2811));
    builder.line_to(point(414.0668, 100.19338));
    builder.line_to(point(271.1228, 536.9269));
    builder.line_to(point(426.74518, 288.92886));
    builder.line_to(point(289.61765, 645.9055));
    builder.line_to(point(20.165205, 351.42136));
    builder.line_to(point(636.5303, 323.59995));
    builder.line_to(point(935.8127, 626.26996));
    builder.line_to(point(7.418394, 328.09793));
    builder.line_to(point(489.52783, 733.52765));
    builder.line_to(point(498.5074, 682.8961));
    builder.line_to(point(159.6706, 248.16739));
    builder.line_to(point(377.1483, 940.8728));
    builder.line_to(point(457.18134, 778.78296));
    builder.line_to(point(104.180214, 324.80096));
    builder.line_to(point(778.18286, 208.72926));
    builder.line_to(point(336.46918, 645.95056));
    builder.line_to(point(95.98338, 8.636117));
    builder.line_to(point(330.69205, 291.2035));
    builder.line_to(point(588.3315, 422.37854));
    builder.close();

    builder.move_to(point(488.39163, 585.3933));
    builder.line_to(point(511.4907, 182.28484));
    builder.line_to(point(207.47495, 267.26733));
    builder.line_to(point(230.20506, 547.68085));
    builder.line_to(point(641.10675, 410.24362));
    builder.line_to(point(256.76678, 199.90837));
    builder.line_to(point(693.10846, 642.2658));
    builder.line_to(point(436.7007, 610.3779));
    builder.line_to(point(522.02405, 973.62683));
    builder.line_to(point(677.5639, 21.071196));
    builder.line_to(point(185.40717, 585.7684));
    builder.line_to(point(865.59296, 169.97707));
    builder.line_to(point(273.83972, 919.5908));
    builder.line_to(point(876.053, 168.97417));
    builder.line_to(point(678.8021, 47.07539));
    builder.line_to(point(722.6765, 159.79457));
    builder.line_to(point(48.471092, 854.4502));
    builder.line_to(point(528.82434, 691.9617));
    builder.line_to(point(234.8243, 171.4369));
    builder.line_to(point(416.02243, 896.4616));
    builder.line_to(point(527.6498, 539.7764));
    builder.line_to(point(672.40405, 646.45374));
    builder.line_to(point(361.3118, 539.9704));
    builder.line_to(point(490.2208, 568.8304));
    builder.line_to(point(419.40283, 91.13407));
    builder.line_to(point(32.00376, 810.6302));
    builder.line_to(point(955.0769, 498.2283));
    builder.line_to(point(493.3964, 146.49857));
    builder.line_to(point(508.61465, 538.3645));
    builder.line_to(point(41.07058, 444.15784));
    builder.line_to(point(194.72015, 70.75846));
    builder.line_to(point(341.9323, 637.0733));
    builder.line_to(point(590.41724, 885.02405));
    builder.line_to(point(634.80115, 146.93617));
    builder.line_to(point(112.83624, 555.17505));
    builder.line_to(point(74.68116, 530.418));
    builder.line_to(point(446.96127, 369.93158));
    builder.line_to(point(626.4776, 614.45166));
    builder.line_to(point(357.09344, 404.3145));
    builder.line_to(point(7.777691, 764.90674));
    builder.line_to(point(991.0443, 899.16394));
    builder.line_to(point(8.897662, 321.36823));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_3() {
    let mut builder = Path::builder();

    builder.move_to(point(618.4506, 535.154));
    builder.line_to(point(316.216, 147.39215));
    builder.line_to(point(865.08704, 400.41245));
    builder.line_to(point(569.64087, 800.3887));
    builder.line_to(point(721.5582, 465.7719));
    builder.line_to(point(271.21198, 23.67413));
    builder.line_to(point(141.09564, 551.85223));
    builder.line_to(point(99.32542, 539.7853));
    builder.line_to(point(385.5622, 93.48655));
    builder.line_to(point(582.3861, 313.66693));
    builder.close();

    builder.move_to(point(543.83075, 16.76654));
    builder.line_to(point(280.80618, 217.78273));
    builder.line_to(point(616.4985, 320.1716));
    builder.line_to(point(988.8271, 24.366737));
    builder.line_to(point(217.42583, 121.19055));
    builder.line_to(point(277.98914, 13.265371));
    builder.line_to(point(459.57483, 478.13153));
    builder.line_to(point(316.33377, 853.58));
    builder.line_to(point(151.98923, 39.224266));
    builder.line_to(point(181.1322, 225.40283));
    builder.line_to(point(903.77435, 602.7105));
    builder.line_to(point(153.47314, 375.58127));
    builder.close();

    builder.move_to(point(419.81577, 161.57126));
    builder.line_to(point(792.4049, 316.38705));
    builder.line_to(point(313.65848, 115.52262));
    builder.line_to(point(724.5401, 494.85623));
    builder.line_to(point(761.3977, 883.6222));
    builder.line_to(point(253.74388, 125.26703));
    builder.line_to(point(887.2149, 296.896));
    builder.close();

    builder.move_to(point(492.68604, 396.9183));
    builder.line_to(point(401.23856, 3.8661957));
    builder.line_to(point(7.4135065, 557.9556));
    builder.line_to(point(960.5184, 431.21362));
    builder.line_to(point(357.84244, 43.77198));
    builder.line_to(point(267.17722, 698.9257));
    builder.close();

    builder.move_to(point(300.043, 142.35281));
    builder.line_to(point(819.65137, 329.5263));
    builder.line_to(point(499.91537, 648.6261));
    builder.line_to(point(479.54404, 118.61658));
    builder.line_to(point(550.64795, 996.7805));
    builder.close();

    builder.move_to(point(858.48926, 195.46806));
    builder.line_to(point(878.9279, 955.1468));
    builder.line_to(point(244.70807, 148.27704));
    builder.line_to(point(670.0153, 170.3571));
    builder.line_to(point(315.85205, 174.97028));
    builder.line_to(point(213.74774, 67.361115));
    builder.line_to(point(766.11206, 286.0285));
    builder.line_to(point(379.79208, 384.93417));
    builder.line_to(point(407.7463, 4.680276));
    builder.line_to(point(430.8783, 381.18555));
    builder.line_to(point(881.6496, 199.04674));
    builder.line_to(point(648.9603, 32.69982));
    builder.close();

    builder.move_to(point(278.53656, 125.74196));
    builder.line_to(point(523.5966, 581.46954));
    builder.line_to(point(20.387054, 433.33923));
    builder.line_to(point(950.0582, 3.176093));
    builder.line_to(point(821.16486, 898.3371));
    builder.line_to(point(144.925, 57.357788));
    builder.line_to(point(895.85876, 92.7962));
    builder.line_to(point(238.99866, 923.9617));
    builder.line_to(point(5.581856, 90.48879));
    builder.line_to(point(424.62277, 187.09552));
    builder.line_to(point(547.2676, 91.477394));
    builder.line_to(point(943.3191, 90.633514));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_4() {
    // This test has a vertex almost on an edge, on its left.
    // The loop that identifies the active edges connected to
    // the current vertex was registering the vertex but skipped
    // the touching edge on its immediate right.
    // The bug was fixed by continuing to iterate over the active
    // edges in this loop and remove the second loop that was not
    // properly testing for touching edges.
    let mut builder = Path::builder();

    builder.move_to(point(953.18604, 567.57916));
    builder.line_to(point(149.4881, 273.67114));
    builder.line_to(point(643.7377, 436.15567));
    builder.close();

    builder.move_to(point(605.66626, 136.37721));
    builder.line_to(point(710.2989, 960.26587));
    builder.line_to(point(473.67264, 879.073));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_5() {
    let mut builder = Path::builder();

    builder.move_to(point(280.44034, 16.12854));
    builder.line_to(point(317.7893, 459.68353));
    builder.line_to(point(267.34244, 17.728329));
    builder.line_to(point(656.3035, 126.39856));
    builder.line_to(point(373.27957, 90.56401));
    builder.line_to(point(20.245314, 715.9369));
    builder.line_to(point(138.39507, 131.92189));
    builder.line_to(point(599.403, 637.9332));
    builder.line_to(point(71.63012, 109.90965));
    builder.line_to(point(369.259, 677.46436));
    builder.line_to(point(440.13644, 702.98157));
    builder.line_to(point(4.911661, 226.04358));
    builder.line_to(point(831.2118, 817.4058));
    builder.line_to(point(755.3699, 812.03796));
    builder.line_to(point(79.84316, 340.46912));
    builder.line_to(point(617.79913, 614.2463));
    builder.close();

    builder.move_to(point(252.10011, 189.48198));
    builder.line_to(point(659.73224, 645.6623));
    builder.line_to(point(22.57371, 656.8426));
    builder.line_to(point(568.11584, 157.38916));
    builder.line_to(point(746.6457, 565.2523));
    builder.line_to(point(328.3987, 24.919628));
    builder.line_to(point(96.115234, 698.0083));
    builder.line_to(point(7.1537495, 530.7252));
    builder.line_to(point(21.84856, 302.0538));
    builder.line_to(point(34.8227, 193.36272));
    builder.close();

    builder.move_to(point(839.4768, 878.84283));
    builder.line_to(point(265.41888, 365.45013));
    builder.line_to(point(678.88605, 98.7531));
    builder.line_to(point(192.53146, 780.74335));
    builder.line_to(point(90.92653, 192.44206));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_6() {
    let mut builder = Path::builder();
    // This test case has a point that is very close on the left of
    // an edge. the loop that finds connected edges above was stopping
    // prematurely because find_interesting_active_edges could sometimes
    // indicate that the point is both on the edge and left of it.

    builder.move_to(point(908.77045, 59.34178));
    builder.line_to(point(177.41656, 803.875));
    builder.line_to(point(803.30835, 166.7068));
    builder.line_to(point(910.1411, 409.8233));
    builder.line_to(point(113.08825, 838.0237));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_7() {
    let mut builder = Path::builder();

    builder.move_to(point(113.852264, 563.1574));
    builder.line_to(point(486.71103, 73.901535));
    builder.line_to(point(705.56006, 835.71826));
    builder.line_to(point(358.2251, 418.4035));
    builder.line_to(point(837.3598, 151.83974));
    builder.close();

    builder.move_to(point(359.5538, 4.9495697));
    builder.line_to(point(825.8098, 129.8927));
    builder.line_to(point(389.28534, 429.343));
    builder.line_to(point(968.47296, 238.33));
    builder.line_to(point(371.02557, 307.2325));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_8() {
    let mut builder = Path::builder();
    // This test a rather complex shape with plenty of intersections
    // including three lines intersecting very close to a certain point.
    // The failure was fixed by increasing the threshold in compare_edge_against_position.

    builder.move_to(point(786.3492, 715.7762));
    builder.line_to(point(108.706955, 396.7073));
    builder.line_to(point(744.5795, 645.1025));
    builder.line_to(point(359.92264, 194.16666));
    builder.line_to(point(432.9413, 690.4683));
    builder.line_to(point(592.9548, 277.76956));
    builder.line_to(point(145.36989, 641.0073));
    builder.close();

    builder.move_to(point(608.8108, 554.82874));
    builder.line_to(point(215.48784, 523.1583));
    builder.line_to(point(821.7586, 872.91003));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_9() {
    let mut builder = Path::builder();
    // This test exercises the usual precision robustness with a vertex
    // very close to an edge.
    // It was fixed by adjusting the threshold in compare_edge_against_position.

    builder.move_to(point(659.9835, 415.86328));
    builder.line_to(point(70.36328, 204.36978));
    builder.line_to(point(74.12529, 89.01107));
    builder.close();

    builder.move_to(point(840.2258, 295.46188));
    builder.line_to(point(259.41193, 272.18054));
    builder.line_to(point(728.914, 281.41678));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_10() {
    let mut builder = Path::builder();

    builder.move_to(point(29.138443, 706.1346));
    builder.line_to(point(347.19098, 7.499695));
    builder.line_to(point(943.01306, 619.71893));
    builder.line_to(point(94.4196, 562.7375));
    builder.line_to(point(569.1717, 605.43097));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_11() {
    let mut builder = Path::builder();

    // 3 segments intersect very close to (329.9366912841797,614.3472747802734).
    // The fix was to bump the snapping threshold up to 24 in compare_edge_against_position.

    builder.move_to(point(626.85846, 976.155));
    builder.line_to(point(200.21939, 393.71896));
    builder.line_to(point(261.13367, 789.74426));
    builder.line_to(point(463.53662, 273.76627));
    builder.line_to(point(690.73224, 841.4799));
    builder.line_to(point(162.06873, 508.66888));
    builder.line_to(point(958.7871, 240.41963));
    builder.line_to(point(172.95158, 566.25415));
    builder.line_to(point(215.60406, 610.8441));
    builder.line_to(point(802.26874, 628.8196));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn fuzzing_test_case_12() {
    let mut builder = Path::builder();

    builder.move_to(point(759.9981, 59.831726));
    builder.line_to(point(960.42285, 418.38144));
    builder.line_to(point(912.67645, 193.0542));
    builder.line_to(point(74.49103, 176.2433));
    builder.line_to(point(542.925, 579.97253));
    builder.line_to(point(920.04016, 75.902466));
    builder.line_to(point(658.5332, 792.19904));
    builder.line_to(point(134.72163, 905.7226));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 759.9981 59.831738 L 960.42285 418.38144 L 912.67645 193.0542 L 74.49103 176.2433 L 542.925 579.97253 L 920.04016 75.902466 L 658.5332 792.19904 L 134.72163 905.7226 Z"
}

#[test]
fn fuzzing_test_case_13() {
    let mut builder = Path::builder();

    // There are some very close almost horizontal segments somewhere around
    // y=773, most likely causing some floating point errors.

    builder.move_to(point(410.68304, 821.1684));
    builder.line_to(point(930.137, 143.92328));
    builder.line_to(point(104.892136, 433.69412));
    builder.line_to(point(660.3361, 814.7637));
    builder.line_to(point(677.3176, 775.74384));
    builder.line_to(point(1.0851622, 766.8102));
    builder.line_to(point(422.32645, 774.1579));
    builder.line_to(point(965.11993, 775.9433));
    builder.line_to(point(543.46405, 972.5189));
    builder.line_to(point(498.56973, 739.5371));
    builder.line_to(point(59.104202, 990.2475));
    builder.line_to(point(222.88525, 571.51117));
    builder.line_to(point(454.01312, 816.9873));
    builder.line_to(point(219.92206, 961.8081));
    builder.line_to(point(198.50409, 103.8456));
    builder.line_to(point(409.76535, 863.5788));
    builder.line_to(point(273.72992, 489.06696));
    builder.line_to(point(479.42303, 773.7393));
    builder.line_to(point(61.974644, 866.6973));
    builder.line_to(point(769.39044, 347.60333));
    builder.line_to(point(594.88464, 818.56824));
    builder.line_to(point(36.028625, 811.2928));
    builder.line_to(point(333.66275, 314.22592));
    builder.line_to(point(110.678795, 817.20044));
    builder.line_to(point(303.23447, 681.25366));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 410.68304 821.1684 L 930.137 143.92328 L 104.892136 433.69412 L 660.3361 814.7637 L 677.3176 775.74384 L 1.0851622 766.8102 L 422.32645 774.1579 L 965.11993 775.9433 L 543.46405 972.5189 L 498.56973 739.5371 L 59.104202 990.2475 L 222.88525 571.51117 L 454.01312 816.9873 L 219.92206 961.8081 L 198.50409 103.8456 L 409.76535 863.5788 L 273.72992 489.06696 L 479.42303 773.7393 L 61.974644 866.6973 L 769.39044 347.60333 L 594.88464 818.56824 L 36.028625 811.2928 L 333.66275 314.22592 L 110.678795 817.20044 L 303.23447 681.25366 Z"
}

#[test]
fn fuzzing_test_case_14() {
    let mut builder = Path::builder();

    builder.move_to(point(906.73926, 854.04346));
    builder.line_to(point(631.4635, 795.7506));
    builder.line_to(point(131.3113, 798.18));
    builder.line_to(point(241.9132, 624.7822));
    builder.line_to(point(249.94122, 902.8816));
    builder.line_to(point(304.89624, 135.56683));
    builder.line_to(point(222.20158, 965.973));
    builder.close();

    builder.move_to(point(810.27686, 494.45905));
    builder.line_to(point(158.03587, 894.21405));
    builder.line_to(point(732.6424, 568.0493));
    builder.line_to(point(419.24048, 855.3553));
    builder.line_to(point(547.4272, 73.85397));
    builder.line_to(point(538.3696, 967.55566));
    builder.line_to(point(282.0003, 138.86476));
    builder.line_to(point(92.06009, 702.09216));
    builder.line_to(point(378.43298, 944.1428));
    builder.line_to(point(290.58493, 608.4501));
    builder.line_to(point(277.56857, 830.6742));
    builder.close();

    builder.move_to(point(450.25922, 792.10675));
    builder.line_to(point(776.7185, 58.490036));
    builder.line_to(point(202.77036, 797.8798));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 906.73926 854.04346 L 631.4635 795.7506 L 131.3113 798.18 L 241.9132 624.7822 L 249.94122 902.8816 L 304.89624 135.56683 L 222.20158 965.973 ZM 810.27686 494.45905 L 158.03587 894.21405 L 732.6424 568.0493 L 419.24048 855.3553 L 547.4272 73.85397 L 538.3696 967.55566 L 282.0003 138.86476 L 92.06009 702.09216 L 378.43298 944.1428 L 290.58493 608.4501 L 277.56857 830.6742 ZM 450.25922 792.10675 L 776.7185 58.490036 L 202.77036 797.8798 Z"
}

#[test]
fn fuzzing_test_case_15() {
    let mut builder = Path::builder();

    builder.move_to(point(458.30704, 64.10158));
    builder.line_to(point(53.061844, 909.0564));
    builder.line_to(point(809.2724, 900.3631));
    builder.line_to(point(59.2463, 896.16016));
    builder.line_to(point(355.03995, 899.7729));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 458.30704 64.10158 L 53.061844 909.0564 L 809.2724 900.3631 L 59.2463 896.16016 L 355.03995 899.7729 Z"
}

#[test]
fn fuzzing_test_case_16() {
    let mut builder = Path::builder();

    builder.move_to(point(424.31747, 191.76984));
    builder.line_to(point(201.27774, 381.03378));
    builder.line_to(point(234.58505, 661.9783));
    builder.line_to(point(487.03467, 23.73457));
    builder.line_to(point(443.45712, 513.29065));
    builder.line_to(point(4.9567223, 154.66821));
    builder.line_to(point(533.0118, 476.51398));
    builder.line_to(point(648.49854, 493.68262));
    builder.line_to(point(82.649704, 259.879));
    builder.line_to(point(777.4901, 453.4104));
    builder.line_to(point(916.51355, 68.055984));
    builder.line_to(point(138.34656, 709.06555));
    builder.line_to(point(17.681717, 255.20825));
    builder.line_to(point(690.94977, 480.44455));
    builder.line_to(point(989.64276, 328.359));
    builder.line_to(point(154.73616, 312.6898));
    builder.line_to(point(524.79614, 260.3277));
    builder.line_to(point(34.26361, 862.9552));
    builder.line_to(point(44.73257, 113.598465));
    builder.line_to(point(527.8045, 794.82306));
    builder.line_to(point(846.9895, 471.9932));
    builder.line_to(point(81.67481, 989.1536));
    builder.line_to(point(58.23517, 72.05153));
    builder.line_to(point(414.9412, 485.44943));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 424.31747 191.76984 L 201.27774 381.03378 L 234.58505 661.9783L 487.03467 23.73457 L 443.45712 513.29065 L 4.9567223 154.66821 L533.0118 476.51398 L 648.49854 493.68262 L 82.649704 259.879 L777.4901 453.4104 L 916.51355 68.055984 L 138.34656 709.06555 L17.681717 255.20825 L 690.94977 480.44455 L 989.64276 328.359 L154.73616 312.6898 L 524.79614 260.3277 L 34.26361 862.9552 L44.73257 113.598465 L 527.8045 794.82306 L 846.9895 471.9932 L81.67481 989.1536 L 58.23517 72.05153 L 414.9412 485.44943 Z"
}

#[test]
fn fuzzing_test_case_17() {
    let mut builder = Path::builder();

    builder.move_to(point(80.462814, 526.54364));
    builder.line_to(point(900.2347, 526.31726));
    builder.line_to(point(237.45477, 531.3444));
    builder.close();

    builder.move_to(point(963.6296, 619.75024));
    builder.line_to(point(572.919, 43.936134));
    builder.line_to(point(837.3995, 894.56165));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 80.462814 526.54364 L 900.2347 526.31726 L 237.45477 531.3444 ZM 963.6296 619.75024 L 572.919 43.936134 L 837.3995 894.56165 Z"
}

#[test]
fn fuzzing_test_case_18() {
    let mut builder = Path::builder();

    // This was fixed by re-sorting the active edges when finding intersections.

    builder.move_to(point(447.85165, 671.4307));
    builder.line_to(point(37.19008, 311.777));
    builder.line_to(point(138.24976, 143.74733));
    builder.line_to(point(159.06596, 538.88116));
    builder.close();

    builder.move_to(point(719.6205, 413.18356));
    builder.line_to(point(75.47033, 794.876));
    builder.line_to(point(25.042057, 412.58252));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 447.85165 671.4307 L 37.19008 311.777 L 138.24976 143.74733 L 159.06596 538.88116 ZM 719.6205 413.18356 L 75.47033 794.876 L 25.042057 412.58252 Z"
}

#[test]
fn fuzzing_test_case_19() {
    let mut builder = Path::builder();

    builder.move_to(point(0.5651398, 0.5119934));
    builder.line_to(point(0.10021269, 0.17348659));
    builder.line_to(point(0.77066493, 0.67880523));
    builder.line_to(point(0.48341691, 0.09270251));
    builder.line_to(point(0.053493023, 0.18919432));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 0.5651517 0.5119977 L 0.10021269 0.17348659 L 0.77066493 0.67880523 L 0.48341691 0.09270251 L 0.053493023 0.18919432 Z"
}

#[test]
fn fuzzing_test_case_20() {
    let mut builder = Path::builder();

    builder.move_to(point(300.44553, -951.7151));
    builder.line_to(point(-311.18967, 952.4652));
    builder.line_to(point(-694.0007, 725.4894));
    builder.line_to(point(683.2565, -724.7392));
    builder.line_to(point(-559.072, -832.3412));
    builder.line_to(point(548.32776, 833.09143));
    builder.line_to(point(132.19205, 990.868));
    builder.line_to(point(-142.93622, -990.1178));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 300.44553 -951.7151 L -311.18967 952.4652 L -694.0007 725.4894 L 683.2565 -724.7392 L -559.072 -832.3412 L 548.32776 833.09143 L 132.19205 990.868 L -142.93622 -990.1178 Z"
}

