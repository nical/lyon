use geometry_builder::{VertexBuffers, simple_builder};
use path::{Path, PathSlice};
use path_fill::*;
use path_iterator::PathIterator;
use path_builder::FlatPathBuilder;
use math::*;
use FillVertex as Vertex;

fn tessellate_path(path: PathSlice, log: bool) -> Result<usize, FillError> {
    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tess = FillTessellator::new();
        if log {
            tess.enable_logging();
        }
        try!{
            tess.tessellate_flattened_path(
                path.path_iter().flattened(0.05),
                &FillOptions::default(),
                &mut vertex_builder
            )
        };
    }
    return Ok(buffers.indices.len() / 3);
}

fn test_path(path: PathSlice) {
    let res = ::std::panic::catch_unwind(|| tessellate_path(path, false));

    if res.is_ok() {
        return;
    }

    ::extra::debugging::find_reduced_test_case(
        path,
        &|path: Path| { return tessellate_path(path.as_slice(), false).is_err(); },
    );

    tessellate_path(path, true).unwrap();
    panic!();
}

#[test]
fn fuzzing_test_case_1() {
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
#[ignore]
fn fuzzing_failure_5() {
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
