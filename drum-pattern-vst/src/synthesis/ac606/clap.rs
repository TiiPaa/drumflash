//! Super 606 Clap — audible match for the analog RD-6.
//!
//! Ported from `Clap.hpp` of analogcode's "606-Inspired-Synth-Drums"
//! — MIT License, Copyright (c) 2026 Matthew Fecher (see `LICENSE-MIT.txt`).
//!
//! Four timed noise bursts and a diffuse tail shaped by short
//! measurement-fitted FIR filters. The four color filters share ONE generated
//! noise stream: separate noise made the color changes sound like crossfades
//! instead of one clap opening up.
//!
//! Noise at 0.5 is the fitted clap. Turn it left for the hard body, or right
//! for more air and a wider, denser tail.

use super::common::{clampf, flush_denormal};

mod detail {
    use super::{clampf, flush_denormal};
    use std::sync::OnceLock;

    // The fitted timing and sample-rate conversion are kept in double so the
    // burst edges do not move at the other sample rates.
    pub const REFERENCE_SAMPLE_RATE: f64 = 44100.0;
    pub const MINIMUM_SUPPORTED_SAMPLE_RATE: f64 = 8000.0;
    pub const MAXIMUM_SUPPORTED_SAMPLE_RATE: f64 = 192000.0;
    pub const CLAP_PI: f64 = std::f64::consts::PI;
    pub const T60_TO_TAU: f64 = 6.907755278982137;
    pub const VOICE_DURATION_SECONDS: f64 = 0.301;
    pub const OUTPUT_TRIM: f64 = 0.1793169072680733;
    pub const SATURATION_THRESHOLD: f64 = 3.0;
    pub const AIR_HIGHPASS_HZ: f64 = 3000.0;
    pub const AIR_DENSITY_THRESHOLD: f64 = 0.75;

    pub const COLOR_TAP_COUNT: usize = 192;
    pub const SHORT_ONSETS: [f64; 4] = [0.0, 0.010737, 0.021735, 0.032778];
    pub const TERMINAL_ONSET: f64 = 0.03278;
    pub const SNAP_ONSET: f64 = 0.032778;
    pub const FOUNDATION_ONSET: f64 = 0.165;
    pub const TERMINAL_FLOOR_ONSET: f64 = 0.250;

    // Four short minimum-phase FIRs fitted from hardware section spectra shape
    // one shared generated-noise stream. Burst timing and envelopes are
    // synthesized below.
    pub static OPENING_COLOR: [f32; COLOR_TAP_COUNT] = [
        1.7299991247e-01, 3.8520469294e-01, 3.6505443856e-01, 3.0560679220e-01,
        2.2452826026e-01, 1.3908714920e-01, 9.1927071889e-02, 1.4103916149e-02,
        -3.7942942562e-02, -8.1746751873e-02, -1.0526792061e-01, -1.2205764226e-01,
        -1.4334556915e-01, -1.5716007865e-01, -1.7842421430e-01, -2.0677230638e-01,
        -2.2379942677e-01, -2.1503897132e-01, -1.8576203109e-01, -1.5083509179e-01,
        -1.4098250958e-01, -1.2832528888e-01, -9.3813025596e-02, -6.0017450617e-02,
        -3.1084397093e-02, -9.4701167885e-03, -7.7901199547e-03, 7.1781049559e-03,
        3.6756277094e-02, 4.5058386570e-02, 5.3116535332e-02, 6.3101344941e-02,
        7.1431644979e-02, 7.8671795335e-02, 6.8710976373e-02, 5.4966574939e-02,
        5.4009053330e-02, 5.7502796101e-02, 7.0540598859e-02, 7.3996216522e-02,
        7.1787348948e-02, 7.4131139235e-02, 5.9788514933e-02, 5.1424890013e-02,
        4.7546079593e-02, 4.4829685002e-02, 3.3747960232e-02, 2.0824402508e-02,
        2.4901738822e-02, 3.5475715459e-02, 2.5897916476e-02, -4.5427778201e-03,
        -1.9814509444e-02, -4.7212428528e-02, -6.5262464268e-02, -7.1616739457e-02,
        -9.8859981673e-02, -1.0089199331e-01, -7.2349242843e-02, -5.3079429834e-02,
        -5.2444421821e-02, -5.2766118080e-02, -5.5103536933e-02, -4.3079985895e-02,
        -2.2298520083e-02, -1.5277499429e-02, -2.0432668089e-03, 3.2518791317e-03,
        -2.1290429544e-03, 4.4082757834e-03, 3.8510594680e-03, 9.2288308501e-04,
        1.4605638148e-02, 2.5834777747e-02, 3.2312073687e-02, 4.1136814026e-02,
        4.3009171598e-02, 3.5981571251e-02, 2.5953073495e-02, 1.8527830369e-02,
        1.3205486210e-02, 7.1091980192e-03, 1.0396742207e-02, 2.0931430046e-02,
        3.1227821414e-02, 4.9580991890e-02, 5.4667524262e-02, 4.5332436846e-02,
        3.1082430763e-02, 1.1505544386e-02, 4.0277029028e-03, -5.4587185574e-03,
        -1.3768563198e-02, -1.4379626752e-02, -1.7033045254e-02, -1.8838404111e-02,
        -2.0550629544e-02, -2.1246278508e-02, -2.6315479921e-02, -2.9134645063e-02,
        -3.1006974644e-02, -3.1152424500e-02, -2.1434461031e-02, -2.0322271955e-02,
        -2.4219789670e-02, -1.9885234655e-02, -1.1013383269e-02, 7.9772630186e-03,
        2.0226216757e-02, 9.8359903528e-03, -2.6826299890e-03, -7.5124332925e-03,
        -1.0426829419e-02, -1.7244872720e-02, -2.2549015083e-02, -1.5310424013e-02,
        -3.7725572308e-03, -6.8477817438e-04, -2.7986644382e-03, -1.7004582564e-03,
        6.2946059616e-03, 9.2504519056e-03, 6.7589491506e-03, 1.3722494372e-02,
        1.9039073252e-02, 2.2172265663e-02, 2.8985564843e-02, 3.0641273329e-02,
        2.8861809496e-02, 2.7316314986e-02, 2.0445908089e-02, 1.3581485055e-02,
        1.5115719828e-02, 1.5888362331e-02, 1.0698867310e-02, 2.8339802856e-03,
        -3.2350045727e-03, -8.5771050159e-03, -1.3926923145e-02, -1.8129481739e-02,
        -2.1631097326e-02, -2.1335178464e-02, -1.8788879538e-02, -1.4653481790e-02,
        -8.9357048749e-03, -6.2253231116e-03, -7.2436394306e-03, -9.2222284343e-03,
        -9.9267520476e-03, -1.0635525159e-02, -1.0652959172e-02, -8.2489791203e-03,
        -9.5926427013e-03, -9.7875005383e-03, -3.8607442057e-03, 3.8359976765e-04,
        4.0924949739e-03, 4.2494315822e-03, 2.5127958410e-03, 6.7434326157e-03,
        8.9475745172e-03, 6.7927039824e-03, 4.4936897603e-03, 2.5254862378e-03,
        3.6229308800e-03, 7.0011716518e-03, 8.7319443032e-03, 8.9431225322e-03,
        8.5529789044e-03, 7.2768236186e-03, 6.1263372792e-03, 5.2646360283e-03,
        3.9868230466e-03, 3.0749426049e-03, 2.9882810754e-03, 2.4603606808e-03,
        1.0739149034e-03, -1.9589900060e-04, -1.4336608958e-03, -2.2916974318e-03,
        -2.2258638497e-03, -1.8234234952e-03, -1.2839692385e-03, -9.3576372805e-04,
        -7.9497181702e-04, -6.1005499408e-04, -4.0669433945e-04, -2.1851935469e-04,
        -8.8834898005e-05, -2.5824787821e-05, -4.9454475024e-06, -0.0000000000e+00,
    ];

    pub static HANDOFF_COLOR: [f32; COLOR_TAP_COUNT] = [
        1.8875619402e-01, 3.9532535099e-01, 3.1052115012e-01, 2.0417681294e-01,
        1.2327780667e-01, 5.3936258901e-02, 2.2253809872e-02, -4.6593291093e-02,
        -1.0347847143e-01, -1.6319263581e-01, -1.8598616783e-01, -1.9691378438e-01,
        -1.9998351978e-01, -1.9882124185e-01, -1.9398388684e-01, -1.7466283449e-01,
        -1.7442871397e-01, -1.6680812656e-01, -1.4932833745e-01, -1.1161678039e-01,
        -7.9639914597e-02, -6.0494803220e-02, -4.3097509189e-02, -3.7946681895e-02,
        -2.7856113623e-02, -4.0528471275e-03, 2.4570897462e-02, 5.1935184672e-02,
        7.3481992541e-02, 8.0365700782e-02, 9.3774064874e-02, 1.1429755485e-01,
        1.2814304655e-01, 1.3704431024e-01, 1.3654880269e-01, 1.3384377070e-01,
        1.2377069562e-01, 1.1334862119e-01, 1.1755749462e-01, 1.1280667005e-01,
        8.8223363515e-02, 6.0171909313e-02, 4.3882991093e-02, 3.1759968057e-02,
        1.0557516928e-02, 6.3407390303e-03, 2.2863515741e-02, 1.7511511564e-02,
        -1.1691636252e-02, -4.0678974631e-02, -5.9415059590e-02, -6.6718975025e-02,
        -6.6513089303e-02, -5.9587464418e-02, -6.1848135339e-02, -7.0400122558e-02,
        -7.4952977630e-02, -8.0457044658e-02, -7.7644015860e-02, -6.1188557650e-02,
        -4.1233682302e-02, -3.3080606371e-02, -3.5374527778e-02, -4.1106987629e-02,
        -4.0476444817e-02, -3.6943129798e-02, -3.7512208948e-02, -3.9119038538e-02,
        -3.6975340594e-02, -1.4263153464e-02, 8.0800135068e-03, 1.7602733217e-02,
        2.2936607116e-02, 2.2241988585e-02, 2.4327434638e-02, 3.5654667183e-02,
        5.1179527021e-02, 5.0305826316e-02, 3.9597608314e-02, 3.1995287216e-02,
        2.8936693434e-02, 2.9734677245e-02, 2.6427212406e-02, 2.7868867792e-02,
        3.0683092212e-02, 2.8941365711e-02, 2.5365597524e-02, 1.6343393416e-02,
        8.8617725898e-03, 4.0539345935e-03, 3.2316595056e-04, -5.5273412411e-04,
        3.0139986806e-04, 6.4484454176e-04, -2.5984268478e-03, -7.3756520558e-03,
        -1.7241810261e-02, -1.6335407228e-02, -7.2853057099e-03, -1.0271613210e-02,
        -8.2024109995e-03, -5.8863433646e-03, -1.0774098558e-02, -1.1499064632e-02,
        -1.0340341879e-03, 1.9238286584e-02, 2.4192210920e-02, 1.0727129830e-02,
        2.4777733322e-03, 1.0215582235e-03, 2.4551051176e-03, 9.3889916913e-03,
        4.0282178255e-03, -8.7561536902e-03, -4.9092698784e-03, -8.6139566513e-04,
        -9.5470480259e-03, -2.1571357383e-02, -2.8191072376e-02, -2.7298650306e-02,
        -2.4646025014e-02, -2.2188370626e-02, -2.2663653592e-02, -1.6921077700e-02,
        -5.8783140947e-03, -3.0692155482e-03, -2.5810634026e-03, -3.0547516211e-03,
        -2.0676771688e-03, -2.2964005711e-03, -7.0261418575e-03, -8.6823902664e-03,
        -1.3690616010e-02, -1.5594866246e-02, -1.5848685375e-02, -1.4647070009e-02,
        2.1477154053e-04, 1.7655075723e-02, 3.0585570046e-02, 3.5469789746e-02,
        3.3094484497e-02, 2.6992796577e-02, 2.6424372263e-02, 3.1620824835e-02,
        3.1452315987e-02, 2.4288905438e-02, 2.1992004564e-02, 2.9756968347e-02,
        2.6089126997e-02, 2.1342070492e-02, 2.2254626751e-02, 1.9769410909e-02,
        1.6155106566e-02, 4.3064050354e-03, -6.2038642358e-03, -1.0095456038e-02,
        -1.3423823436e-02, -1.9278293978e-02, -1.9673820252e-02, -1.1910960372e-02,
        -1.3594786202e-02, -2.1142244028e-02, -2.7204330424e-02, -3.1068176630e-02,
        -2.7894902200e-02, -2.7153380416e-02, -2.8041008611e-02, -2.4219212736e-02,
        -2.0634875707e-02, -1.6187012168e-02, -1.0479424511e-02, -7.2663527681e-03,
        -5.6374416729e-03, -3.7422698206e-03, -1.4163680254e-03, 3.3928004566e-04,
        2.3420011879e-03, 4.7162152723e-03, 5.4279575118e-03, 5.5645376352e-03,
        5.5513861468e-03, 4.3937996309e-03, 3.2511581219e-03, 2.9783092063e-03,
        2.7951769845e-03, 2.2479299804e-03, 1.3995877132e-03, 6.9904864455e-04,
        3.6704890385e-04, 1.7713908110e-04, 3.7858337451e-05, 0.0000000000e+00,
    ];

    pub static FAST_COLOR: [f32; COLOR_TAP_COUNT] = [
        1.4644279033e-01, 3.3251425492e-01, 3.0859052842e-01, 2.3850248802e-01,
        1.7106916088e-01, 8.8562854779e-02, 3.1893734688e-02, -3.0031178833e-02,
        -7.9945965465e-02, -1.3244572685e-01, -1.6362719173e-01, -1.7710008278e-01,
        -1.9040712613e-01, -2.0589490809e-01, -2.1188769343e-01, -2.0689380767e-01,
        -2.0043829364e-01, -1.8881694375e-01, -1.7639896033e-01, -1.5042825626e-01,
        -1.1744597235e-01, -7.8919664332e-02, -4.4104005328e-02, -3.8904541670e-03,
        3.0533000837e-02, 4.4945801627e-02, 6.5836836767e-02, 8.2453527556e-02,
        9.2875174988e-02, 9.7217019887e-02, 1.0303531878e-01, 1.0783131664e-01,
        1.0028010263e-01, 9.9973070631e-02, 9.5708973060e-02, 8.3961725978e-02,
        8.0039703641e-02, 8.8430083766e-02, 9.5913529808e-02, 8.7543081064e-02,
        7.9121999752e-02, 7.5578302503e-02, 7.1182878368e-02, 5.7252560998e-02,
        3.9309422778e-02, 2.5171764280e-02, 4.4653573320e-03, -1.1864384777e-02,
        -3.0366262986e-02, -5.5890672791e-02, -7.5085014848e-02, -8.6867688815e-02,
        -8.8778224349e-02, -9.0584711493e-02, -9.2215571744e-02, -9.1266542927e-02,
        -9.2624401986e-02, -9.6245312917e-02, -8.9648770150e-02, -6.9226921724e-02,
        -5.5988772723e-02, -4.4530985568e-02, -2.6720085666e-02, -7.9709798946e-03,
        9.1258579198e-03, 2.3305216710e-02, 3.2412006527e-02, 3.3089622507e-02,
        3.5608404131e-02, 3.9076460601e-02, 3.4604297928e-02, 3.3155923189e-02,
        4.1606745316e-02, 4.6396513320e-02, 4.8867472567e-02, 5.6108334898e-02,
        6.0791044239e-02, 6.1414992382e-02, 5.3634383802e-02, 4.0484233121e-02,
        2.5334577443e-02, 6.9592828718e-03, -5.5072231193e-03, -1.3580403470e-02,
        -2.4465813753e-02, -3.0377069583e-02, -2.5157539555e-02, -2.2014305929e-02,
        -1.8436730819e-02, -1.3259652274e-02, -1.5716725316e-02, -1.5201767541e-02,
        -1.0464498135e-02, -1.0328027209e-02, -1.3266340203e-02, -9.3307773966e-03,
        -3.3698780013e-03, 2.2331068473e-03, 8.3534615478e-03, 1.1517625483e-02,
        1.1225234035e-02, 5.6612264261e-03, 8.4687115795e-03, 1.1962385205e-02,
        5.6750515076e-03, -3.4196079220e-03, -1.1423524889e-02, -1.5239401905e-02,
        -1.6309600843e-02, -1.7943625745e-02, -1.8068378795e-02, -8.8903004155e-03,
        -3.5731864575e-03, -5.5923071192e-03, -7.9615682441e-03, -1.1019803958e-02,
        -8.3741907120e-03, -6.0801361338e-03, -5.9358203047e-03, -4.4675137155e-03,
        -5.5096303097e-03, -7.7904723738e-04, 1.1686518541e-02, 1.5498594718e-02,
        1.1043152991e-02, 6.8694248426e-03, 3.9785150408e-03, 5.6377612554e-03,
        7.6844872390e-03, 9.0475647339e-03, 1.3301744585e-02, 1.8719059063e-02,
        2.3454687156e-02, 2.2704440167e-02, 2.1347777532e-02, 2.1712409763e-02,
        1.7248403430e-02, 1.1262126584e-02, 8.4824704213e-03, 6.0440605959e-03,
        -1.6710228449e-03, -4.4042710423e-03, -1.8068409087e-03, -3.7550599497e-03,
        -5.3673556029e-03, -6.3848212539e-03, -5.3632438609e-03, -4.7760217279e-03,
        -8.7347270831e-03, -1.2969979200e-02, -1.4830935442e-02, -1.3895969576e-02,
        -1.5001843702e-02, -1.7324794169e-02, -1.7296900826e-02, -1.8139915010e-02,
        -2.1433352777e-02, -2.6018132861e-02, -2.6095548860e-02, -1.9520000378e-02,
        -1.1810525169e-02, -6.4036120259e-03, -2.0758811410e-03, 1.4735475877e-03,
        4.3694484793e-03, 9.1458877127e-03, 1.1001737153e-02, 9.1675550847e-03,
        8.1882326218e-03, 8.3674704541e-03, 9.4950997703e-03, 9.4993343768e-03,
        9.3898558781e-03, 9.0901648646e-03, 7.2145556756e-03, 5.2846953263e-03,
        3.8242149942e-03, 2.2761975244e-03, 9.8110836439e-04, -2.0780521747e-04,
        -1.5306391481e-03, -1.6848704170e-03, -8.8404675258e-04, -3.0776405227e-04,
        -2.2786459061e-04, -1.2461349374e-04, 5.4600496690e-05, 5.5594324186e-05,
        7.2649705392e-06, -9.3334849094e-06, -3.7097946147e-06, -0.0000000000e+00,
    ];

    pub static LATE_COLOR: [f32; COLOR_TAP_COUNT] = [
        8.7130201318e-02, 2.2155809298e-01, 2.8348361607e-01, 2.8969521695e-01,
        2.5567232669e-01, 2.0070052321e-01, 1.3676317259e-01, 7.4221264546e-02,
        2.1740344735e-02, -3.2235552780e-02, -7.9975832576e-02, -1.2418344686e-01,
        -1.5733516311e-01, -1.8224817432e-01, -2.0354665656e-01, -2.1595437326e-01,
        -2.1840114540e-01, -2.1408287478e-01, -2.0775063975e-01, -1.9588524792e-01,
        -1.7977876415e-01, -1.5828956198e-01, -1.3631974334e-01, -1.1430055582e-01,
        -8.9797315789e-02, -6.2661166421e-02, -3.5405955397e-02, -1.3159330961e-02,
        9.6000203231e-03, 3.1611400021e-02, 5.1726064218e-02, 6.8934579219e-02,
        8.2731548195e-02, 9.3974995507e-02, 1.0207550769e-01, 1.1138532055e-01,
        1.1672223674e-01, 1.1800638420e-01, 1.1939892180e-01, 1.1432582188e-01,
        1.0506392496e-01, 9.6589559732e-02, 8.8206046829e-02, 7.9076695511e-02,
        6.6358888242e-02, 5.2976560198e-02, 4.0920832018e-02, 2.7033126345e-02,
        1.2788449284e-02, -8.3522389302e-04, -1.4258011163e-02, -2.7707175256e-02,
        -4.0898501891e-02, -5.1405484331e-02, -5.7644255084e-02, -6.1241170245e-02,
        -6.4352140841e-02, -6.6777290497e-02, -6.8791522233e-02, -6.8268254727e-02,
        -6.2136623545e-02, -4.9844425610e-02, -3.6728842541e-02, -2.7176030821e-02,
        -1.9985779258e-02, -1.6729785576e-02, -1.0199577848e-02, 1.1963622227e-03,
        1.0580382277e-02, 1.7103609343e-02, 2.0289546288e-02, 2.2227453956e-02,
        2.3310296858e-02, 2.5875142251e-02, 3.0148347540e-02, 3.2651095324e-02,
        3.4763920863e-02, 3.5975117165e-02, 3.4584726143e-02, 3.1687552408e-02,
        2.8861034451e-02, 2.3490087772e-02, 1.6815260403e-02, 1.0886991352e-02,
        5.1574091197e-03, 1.0787372940e-03, -4.2088313830e-03, -9.1105071966e-03,
        -1.2059160842e-02, -1.4331597218e-02, -1.4107396181e-02, -1.4870141096e-02,
        -1.8241726463e-02, -2.1682008998e-02, -2.4036347026e-02, -2.3494730691e-02,
        -2.0688706205e-02, -1.8326889810e-02, -1.5230415289e-02, -1.1777225703e-02,
        -7.1524386733e-03, -1.4101943252e-03, 1.5418741362e-03, 3.6459285113e-03,
        6.2476352278e-03, 7.7223464474e-03, 8.3337695167e-03, 9.4597559187e-03,
        1.0568134465e-02, 9.2203000562e-03, 7.7240427661e-03, 8.7404367115e-03,
        9.0389128719e-03, 7.5681684818e-03, 6.3264442002e-03, 5.4600726174e-03,
        3.8208311955e-03, 2.8758134510e-03, 4.2005461535e-03, 5.4062053862e-03,
        6.4444727542e-03, 6.8311808173e-03, 5.4200081292e-03, 3.6673254723e-03,
        9.4138177350e-04, -1.2776751178e-03, -1.3783768568e-03, 2.5254776593e-04,
        2.4141877170e-03, 2.7787826027e-03, 8.1125846429e-04, -2.2762711770e-03,
        -4.2076426997e-03, -5.3592031942e-03, -7.0290615545e-03, -7.5847080386e-03,
        -6.6053000721e-03, -5.2488755389e-03, -5.3682239908e-03, -5.9946908463e-03,
        -6.0427003775e-03, -7.1768089447e-03, -7.1268693491e-03, -4.6158791520e-03,
        -2.6794016852e-03, -1.3168045105e-03, 1.0162617713e-03, 2.7650499789e-03,
        3.7682566833e-03, 4.4813817320e-03, 3.7970185695e-03, 1.9352859909e-03,
        2.8837791595e-04, 5.9387703962e-04, 1.7123794523e-03, 2.3434977281e-03,
        2.5523219511e-03, 1.3922337427e-03, -1.0847044549e-03, -3.2997248024e-03,
        -3.4236017940e-03, -1.1795445111e-03, 1.0515971477e-03, 9.3306626293e-04,
        8.4823736794e-04, 1.4738633314e-03, 1.7278435695e-03, 1.9500228298e-03,
        1.9098723026e-03, 1.9834121276e-03, 2.1886978555e-03, 2.2751726702e-03,
        2.3278500034e-03, 2.3505952985e-03, 1.7926370794e-03, 9.5787423866e-04,
        4.2225305007e-04, 2.5186489566e-04, 9.1255405584e-05, -3.9853992958e-04,
        -7.7767560106e-04, -8.7674023069e-04, -7.8006074845e-04, -5.7859495911e-04,
        -4.1427615131e-04, -2.7300701604e-04, -1.7115147834e-04, -9.2326998995e-05,
        -4.3627117585e-05, -2.0987719357e-05, -5.3857387243e-06, -0.0000000000e+00,
    ];

    pub struct Random {
        state: u32,
    }

    impl Random {
        pub fn new(seed: u32) -> Self {
            Self {
                state: if seed == 0 { 0x6D2B79F5 } else { seed },
            }
        }

        pub fn gaussianish(&mut self) -> f32 {
            let mut sum = 0.0f64;
            for _ in 0..6 {
                sum += self.bipolar();
            }
            (sum * 0.70710678118654752440) as f32
        }

        fn bipolar(&mut self) -> f64 {
            let mut value = self.state;
            value ^= value << 13;
            value ^= value >> 17;
            value ^= value << 5;
            self.state = value;
            ((value >> 8) as f64) * (2.0 / 16777215.0) - 1.0
        }
    }

    pub struct CompressedSpectralCurve {
        coefficients: [f32; COLOR_TAP_COUNT],
        tap_count: usize,
    }

    impl CompressedSpectralCurve {
        pub fn new() -> Self {
            Self {
                coefficients: [0.0; COLOR_TAP_COUNT],
                tap_count: COLOR_TAP_COUNT,
            }
        }

        pub fn configure(&mut self, source: &[f32; COLOR_TAP_COUNT], source_step: f32) {
            let step = source_step.max(1.0);
            self.tap_count = ((COLOR_TAP_COUNT as f32 / step).ceil() as usize)
                .clamp(1, COLOR_TAP_COUNT);
            if step == 1.0 {
                self.coefficients = *source;
                self.tap_count = COLOR_TAP_COUNT;
                return;
            }

            let mut raw_energy = 0.0f64;
            let mut source_energy = 0.0f64;
            for value in source.iter() {
                source_energy += (*value as f64) * (*value as f64);
            }
            for tap in 0..self.tap_count {
                self.coefficients[tap] = cubic_sample(source, tap as f32 * step);
                raw_energy +=
                    (self.coefficients[tap] as f64) * (self.coefficients[tap] as f64);
            }
            let scale = (source_energy / raw_energy.max(1.0e-20)).sqrt() as f32;
            for tap in 0..self.tap_count {
                self.coefficients[tap] *= scale;
            }
            for tap in self.tap_count..COLOR_TAP_COUNT {
                self.coefficients[tap] = 0.0;
            }
        }

        #[inline]
        pub fn coefficient(&self, tap: usize) -> f32 {
            self.coefficients[tap]
        }

        pub fn tap_count(&self) -> usize {
            self.tap_count
        }
    }

    fn source_sample(source: &[f32; COLOR_TAP_COUNT], index: i32) -> f32 {
        if index >= 0 && (index as usize) < COLOR_TAP_COUNT {
            source[index as usize]
        } else {
            0.0
        }
    }

    fn cubic_sample(source: &[f32; COLOR_TAP_COUNT], position: f32) -> f32 {
        let index = position.floor() as i32;
        let t = position - index as f32;
        let p0 = source_sample(source, index - 1);
        let p1 = source_sample(source, index);
        let p2 = source_sample(source, index + 1);
        let p3 = source_sample(source, index + 2);
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * (2.0 * p1 + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }

    #[derive(Clone, Copy, Default)]
    pub struct ColorFrame {
        pub opening: f32,
        pub handoff: f32,
        pub fast: f32,
        pub late: f32,
    }

    /// Keep the dry body beside the full clap so Noise can change the tail
    /// without starting another random stream.
    #[derive(Clone, Copy, Default)]
    pub struct CoreFrame {
        pub full: f32,
        pub dry_body: f32,
    }

    /// All four filters get the same noise.
    pub struct CorrelatedColorBank {
        opening: CompressedSpectralCurve,
        handoff: CompressedSpectralCurve,
        fast: CompressedSpectralCurve,
        late: CompressedSpectralCurve,
        history: [f32; COLOR_TAP_COUNT],
        tap_count: usize,
        position: usize,
    }

    impl CorrelatedColorBank {
        pub fn new() -> Self {
            Self {
                opening: CompressedSpectralCurve::new(),
                handoff: CompressedSpectralCurve::new(),
                fast: CompressedSpectralCurve::new(),
                late: CompressedSpectralCurve::new(),
                history: [0.0; COLOR_TAP_COUNT],
                tap_count: COLOR_TAP_COUNT,
                position: 0,
            }
        }

        pub fn configure(&mut self, source_step: f32) {
            self.opening.configure(&OPENING_COLOR, source_step);
            self.handoff.configure(&HANDOFF_COLOR, source_step);
            self.fast.configure(&FAST_COLOR, source_step);
            self.late.configure(&LATE_COLOR, source_step);
            self.tap_count = self.opening.tap_count();
            self.history = [0.0; COLOR_TAP_COUNT];
            self.position = 0;
        }

        pub fn prewarm(&mut self, random: &mut Random) {
            // Tune changes the filter length, but it should not change the
            // random starting point.
            for _ in 0..COLOR_TAP_COUNT {
                self.push(random.gaussianish());
            }
        }

        #[inline]
        pub fn process(&mut self, noise: f32) -> ColorFrame {
            self.push(noise);
            let mut output = ColorFrame::default();
            for tap in 0..self.tap_count {
                let index = (self.position + self.tap_count - 1 - tap) % self.tap_count;
                let sample = self.history[index];
                output.opening += self.opening.coefficient(tap) * sample;
                output.handoff += self.handoff.coefficient(tap) * sample;
                output.fast += self.fast.coefficient(tap) * sample;
                output.late += self.late.coefficient(tap) * sample;
            }
            output.opening = flush_denormal(output.opening);
            output.handoff = flush_denormal(output.handoff);
            output.fast = flush_denormal(output.fast);
            output.late = flush_denormal(output.late);
            output
        }

        #[inline]
        fn push(&mut self, sample: f32) {
            self.history[self.position] = flush_denormal(sample);
            self.position = if self.position + 1 == self.tap_count {
                0
            } else {
                self.position + 1
            };
        }
    }

    #[inline]
    pub fn raised_cosine_ramp(time: f64, start: f64, end: f64) -> f64 {
        if time <= start {
            return 0.0;
        }
        if time >= end {
            return 1.0;
        }
        let position = (time - start) / (end - start);
        0.5 - 0.5 * (CLAP_PI * position).cos()
    }

    #[inline]
    pub fn raised_sine_gate(time: f64, onset: f64, attack: f64, hold: f64, t60: f64) -> f64 {
        let age = time - onset;
        if age < 0.0 {
            return 0.0;
        }
        let mut attack_gain = 1.0;
        if attack > 0.0 && age < attack {
            let sine = (0.5 * CLAP_PI * age / attack).sin();
            attack_gain = sine * sine;
        }
        let decay_age = (age - attack - hold).max(0.0);
        attack_gain * (-T60_TO_TAU * decay_age / t60.max(1.0e-6)).exp()
    }

    pub struct ClapCore {
        sample_rate: f64,
        decay: f32,
        air_spread: f32,
        /// [196] Scales the four burst onsets (1.0 = fitted timing).
        spread: f32,
        /// [196] Level of the diffuse terminal/foundation tail (1.0 = fitted).
        tail: f32,
        sample_index: u64,
        random: Random,
        colors: CorrelatedColorBank,
    }

    impl ClapCore {
        pub fn new(seed: u32) -> Self {
            Self {
                sample_rate: REFERENCE_SAMPLE_RATE,
                decay: 1.0,
                air_spread: 0.0,
                spread: 1.0,
                tail: 1.0,
                sample_index: 0,
                random: Random::new(seed),
                colors: CorrelatedColorBank::new(),
            }
        }

        pub fn trigger(
            &mut self,
            sample_rate: f64,
            decay: f32,
            source_step: f32,
            air_spread: f32,
            spread: f32,
            tail: f32,
        ) {
            self.sample_rate = sample_rate;
            self.decay = decay;
            self.air_spread = clampf(air_spread, 0.0, 1.0);
            self.spread = spread.max(0.0);
            self.tail = tail.max(0.0);
            self.colors.configure(source_step);
            self.colors.prewarm(&mut self.random);
            self.sample_index = 0;
        }

        #[inline]
        pub fn process(&mut self) -> CoreFrame {
            let color = self.colors.process(self.random.gaussianish());
            let time = self.sample_index as f64 / self.sample_rate;
            let decay = self.decay as f64;
            let diffuse_decay_scale = 1.0 + 0.45 * self.air_spread as f64;

            const LEVELS: [f64; 4] = [2.00, 1.85, 1.16, 0.46];
            const T60S: [f64; 4] = [0.0125, 0.014, 0.019, 0.025];

            let mut short_envelope = 0.0f64;
            for burst in 0..SHORT_ONSETS.len() {
                short_envelope += LEVELS[burst]
                    * raised_sine_gate(
                        time,
                        SHORT_ONSETS[burst] * self.spread as f64,
                        if burst == 0 { 0.0 } else { 0.00016 },
                        0.00030,
                        T60S[burst] * decay,
                    );
            }
            let mut dry_body = color.opening as f64 * short_envelope;
            let mut output = dry_body;

            let terminal_age = time - TERMINAL_ONSET * self.spread as f64;
            if terminal_age >= 0.0 {
                let terminal_attack = if terminal_age < 0.00020 {
                    let s = (0.5 * CLAP_PI * terminal_age / 0.00020).sin();
                    s * s
                } else {
                    1.0
                };
                let terminal_decay_age = (terminal_age - 0.00020 - 0.019).max(0.0);
                let terminal_envelope = 0.72
                    * terminal_attack
                    * (0.955
                        * (-T60_TO_TAU * terminal_decay_age
                            / ((0.225 * decay) * diffuse_decay_scale))
                            .exp()
                        + 0.045
                            * (-T60_TO_TAU * terminal_decay_age
                                / ((0.890 * decay) * diffuse_decay_scale))
                                .exp());

                let fast_mix = raised_cosine_ramp(time, 0.055, 0.080);
                let mut terminal_color = (1.0 - fast_mix) * color.handoff as f64
                    + fast_mix * color.fast as f64;
                let late_mix = raised_cosine_ramp(time, 0.110, 0.170);
                terminal_color =
                    (1.0 - late_mix) * terminal_color + late_mix * color.late as f64;
                output += self.tail as f64 * terminal_envelope * terminal_color;
            }

            let snap = 0.55
                * color.opening as f64
                * raised_sine_gate(time, SNAP_ONSET * self.spread as f64, 0.00012, 0.00035, 0.025 * decay);
            output += snap;
            dry_body += snap;
            // Keep the two late layers tied to Decay so a short clap does not
            // fade out and pop back in.
            output += self.tail as f64
                * 0.022
                * decay
                * color.late as f64
                * raised_sine_gate(
                    time,
                    FOUNDATION_ONSET,
                    0.020,
                    0.0,
                    (0.760 * decay) * diffuse_decay_scale,
                );
            output += self.tail as f64
                * 0.004
                * decay
                * color.late as f64
                * raised_sine_gate(
                    time,
                    TERMINAL_FLOOR_ONSET,
                    0.015,
                    0.0,
                    (0.760 * decay) * diffuse_decay_scale,
                );

            self.sample_index += 1;
            CoreFrame {
                full: flush_denormal(output as f32),
                dry_body: flush_denormal(dry_body as f32),
            }
        }
    }

    pub const RECONSTRUCTION_PHASE_COUNT: usize = 32;
    pub const RECONSTRUCTION_TAP_COUNT: usize = 128;
    pub const RECONSTRUCTION_CUTOFF: f32 = 0.85;

    /// The clap is tuned at 44.1 kHz. This table keeps the shape together at
    /// the other sample rates without putting an interpolator in each voice.
    pub struct ReconstructionTable {
        pub coefficients: [[f32; RECONSTRUCTION_TAP_COUNT]; RECONSTRUCTION_PHASE_COUNT + 1],
    }

    static RECONSTRUCTION_TABLE: OnceLock<Box<ReconstructionTable>> = OnceLock::new();

    pub fn reconstruction_table() -> &'static ReconstructionTable {
        RECONSTRUCTION_TABLE.get_or_init(|| {
            const FIRST_OFFSET: i32 = -63;
            const RADIUS: f32 = 64.0;
            let mut table = Box::new(ReconstructionTable {
                coefficients: [[0.0; RECONSTRUCTION_TAP_COUNT]; RECONSTRUCTION_PHASE_COUNT + 1],
            });
            for phase in 0..=RECONSTRUCTION_PHASE_COUNT {
                let fraction = phase as f32 / RECONSTRUCTION_PHASE_COUNT as f32;
                let mut sum = 0.0f32;
                for tap in 0..RECONSTRUCTION_TAP_COUNT {
                    let offset = (FIRST_OFFSET + tap as i32) as f32;
                    let distance = fraction - offset;
                    let absolute = distance.abs();
                    let mut value = 0.0f32;
                    if absolute < RADIUS {
                        let argument = std::f32::consts::PI * RECONSTRUCTION_CUTOFF * distance;
                        let sinc = if argument.abs() < 1.0e-7 {
                            1.0
                        } else {
                            argument.sin() / argument
                        };
                        let window = 0.42
                            + 0.50 * (std::f32::consts::PI * distance / RADIUS).cos()
                            + 0.08 * (2.0 * std::f32::consts::PI * distance / RADIUS).cos();
                        value = RECONSTRUCTION_CUTOFF * sinc * window;
                    }
                    table.coefficients[phase][tap] = value;
                    sum += value;
                }
                let inverse = 1.0 / sum.abs().max(1.0e-12);
                for tap in 0..RECONSTRUCTION_TAP_COUNT {
                    table.coefficients[phase][tap] *= inverse;
                }
            }
            table
        })
    }

    pub struct CoreUpsampler {
        history: [CoreFrame; 256],
        generated_samples: u64,
        source_position: f64,
        source_step: f32,
    }

    impl CoreUpsampler {
        pub fn new() -> Self {
            Self {
                history: [CoreFrame::default(); 256],
                generated_samples: 0,
                source_position: 0.0,
                source_step: 1.0,
            }
        }

        pub fn configure(&mut self, source_step: f32) {
            self.source_step = clampf(source_step, 0.0, 1.0);
            self.source_position = 0.0;
            self.generated_samples = 0;
            self.history = [CoreFrame::default(); 256];
        }

        #[inline]
        pub fn process(&mut self, core: &mut ClapCore) -> CoreFrame {
            const FIRST_OFFSET: i64 = -63;
            const LAST_OFFSET: i64 = 64;
            let center = self.source_position.floor() as i64;
            self.ensure_generated(center + LAST_OFFSET, core);
            let fraction = (self.source_position - center as f64) as f32;
            let phase_position = fraction * RECONSTRUCTION_PHASE_COUNT as f32;
            let phase = (phase_position.floor() as usize).min(RECONSTRUCTION_PHASE_COUNT - 1);
            let phase_mix = phase_position - phase as f32;
            let table = reconstruction_table();
            let mut output = CoreFrame::default();
            for offset in FIRST_OFFSET..=LAST_OFFSET {
                let tap = (offset - FIRST_OFFSET) as usize;
                let first = table.coefficients[phase][tap];
                let second = table.coefficients[phase + 1][tap];
                let coefficient = first + phase_mix * (second - first);
                let source = self.source_at(center + offset);
                output.full += coefficient * source.full;
                output.dry_body += coefficient * source.dry_body;
            }
            self.source_position += self.source_step as f64;
            output.full = flush_denormal(output.full);
            output.dry_body = flush_denormal(output.dry_body);
            output
        }

        fn ensure_generated(&mut self, final_index: i64, core: &mut ClapCore) {
            let final_idx = if final_index > 0 { final_index as u64 } else { 0 };
            while self.generated_samples <= final_idx {
                self.history[(self.generated_samples % self.history.len() as u64) as usize] =
                    core.process();
                self.generated_samples += 1;
            }
        }

        fn source_at(&self, index: i64) -> CoreFrame {
            if index < 0 || index as u64 >= self.generated_samples {
                return CoreFrame::default();
            }
            self.history[(index as u64 % self.history.len() as u64) as usize]
        }
    }

    /// Builds the reconstruction table outside the audio thread.
    pub fn prewarm_table() {
        let _ = reconstruction_table();
    }
}

pub use detail::prewarm_table as prewarm;

/// Clap voice wrapper: Noise at 0.5 is the fitted clap.
pub struct AcClap {
    sample_rate: f64,
    active: bool,
    sample_index: u32,
    maximum_samples: u32,
    core_step: f32,
    noise_layer_gain: f32,
    air_spread: f32,
    air_highpass_coefficient: f32,
    air_highpass_input: f32,
    air_highpass_output: f32,
    core: detail::ClapCore,
    upsampler: detail::CoreUpsampler,
}

impl AcClap {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let sr = if sample_rate.is_finite()
            && (sample_rate as f64) >= detail::MINIMUM_SUPPORTED_SAMPLE_RATE
            && (sample_rate as f64) <= detail::MAXIMUM_SUPPORTED_SAMPLE_RATE
        {
            sample_rate as f64
        } else {
            detail::REFERENCE_SAMPLE_RATE
        };
        Self {
            sample_rate: sr,
            active: false,
            sample_index: 0,
            maximum_samples: 0,
            core_step: 1.0,
            noise_layer_gain: 1.0,
            air_spread: 0.0,
            air_highpass_coefficient: 0.0,
            air_highpass_input: 0.0,
            air_highpass_output: 0.0,
            core: detail::ClapCore::new(if seed == 0 { 0x0606C1A9 } else { seed }),
            upsampler: detail::CoreUpsampler::new(),
        }
    }

    /// `decay_percent` 0..1, `pitch_ratio` (1 = original tuning),
    /// `noise_amount` 0..1 (0.5 = the fitted clap),
    /// `spread` burst timing scale (1.0 = fitted), `tail` diffuse tail level
    /// (1.0 = fitted), `air` forced air layer floor (0.0 = fitted).
    pub fn trigger(
        &mut self,
        decay_percent: f32,
        pitch_ratio: f32,
        noise_amount: f32,
        spread: f32,
        tail: f32,
        air: f32,
    ) {
        let safe_decay = if decay_percent.is_finite() {
            clampf(decay_percent, 0.05, 1.0)
        } else {
            1.0
        };
        let ratio = if pitch_ratio.is_finite() {
            clampf(pitch_ratio.abs(), 0.5, 2.0)
        } else {
            1.0
        };
        let normalized_noise = if noise_amount.is_finite() {
            clampf(noise_amount, 0.0, 1.0)
        } else {
            0.5
        };
        // The left half brings in the colored noise. Above center the attack
        // is left alone and brightness/density grow later in the tail.
        self.noise_layer_gain = 2.0 * normalized_noise.min(0.5);
        // [196] `air` forces the spread layer even at Noise <= 0.5.
        let noise_air = if normalized_noise > 0.5 {
            2.0 * (normalized_noise - 0.5)
        } else {
            0.0
        };
        self.air_spread = clampf(noise_air.max(air), 0.0, 1.0);
        let desired_core_rate = detail::REFERENCE_SAMPLE_RATE * ratio as f64;
        let core_rate = desired_core_rate.min(self.sample_rate);
        let color_source_step = (desired_core_rate / core_rate) as f32;
        self.core_step = (core_rate / self.sample_rate) as f32;

        self.core.trigger(
            core_rate,
            safe_decay,
            color_source_step,
            self.air_spread,
            if spread.is_finite() { spread } else { 1.0 },
            if tail.is_finite() { tail } else { 1.0 },
        );
        self.upsampler.configure(self.core_step);
        let air_cutoff = detail::AIR_HIGHPASS_HZ.min(0.35 * self.sample_rate);
        self.air_highpass_coefficient =
            (-2.0 * detail::CLAP_PI * air_cutoff / self.sample_rate).exp() as f32;
        self.air_highpass_input = 0.0;
        self.air_highpass_output = 0.0;
        self.maximum_samples =
            ((self.sample_rate * detail::VOICE_DURATION_SECONDS).ceil() as u32).max(1);
        self.sample_index = 0;
        self.active = true;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let raw = if self.core_step == 1.0 {
            self.core.process()
        } else {
            self.upsampler.process(&mut self.core)
        };
        let time = self.sample_index as f64 / self.sample_rate;
        let fade_position = ((time - 0.290) / (detail::VOICE_DURATION_SECONDS - 0.290))
            .clamp(0.0, 1.0);
        let fade_base = 0.5 + 0.5 * (detail::CLAP_PI * fade_position).cos();
        let fade = fade_base * fade_base * fade_base * fade_base;
        let faded = raw.full as f64 * fade;
        let calibrated =
            detail::SATURATION_THRESHOLD * (faded / detail::SATURATION_THRESHOLD).tanh();
        let mut saturated = calibrated;
        if self.air_spread > 0.0 {
            let dry_faded = raw.dry_body as f64 * fade;
            let dry =
                detail::SATURATION_THRESHOLD * (dry_faded / detail::SATURATION_THRESHOLD).tanh();
            let calibrated_noise = calibrated - dry;
            let residual = calibrated_noise as f32;
            self.air_highpass_output = flush_denormal(
                self.air_highpass_coefficient
                    * (self.air_highpass_output + residual - self.air_highpass_input),
            );
            self.air_highpass_input = flush_denormal(residual);

            // This only pushes the colored part. The first slap stays put
            // while the high noise and compressed wash grow behind it.
            let late_weight = detail::raised_cosine_ramp(time, 0.045, 0.170);
            let spread = self.air_spread as f64;
            let direct_gain = 1.0 + spread * (0.40 + 0.60 * late_weight);
            let density_mix = spread * (0.20 + 0.55 * late_weight);
            let dense_noise = detail::AIR_DENSITY_THRESHOLD
                * (2.0 * calibrated_noise / detail::AIR_DENSITY_THRESHOLD).tanh();
            let air_mix = spread * (0.15 + 0.50 * late_weight);
            saturated = dry
                + direct_gain * calibrated_noise
                + density_mix * dense_noise
                + air_mix * self.air_highpass_output as f64;
        } else if self.noise_layer_gain != 1.0 {
            let dry_faded = raw.dry_body as f64 * fade;
            let dry =
                detail::SATURATION_THRESHOLD * (dry_faded / detail::SATURATION_THRESHOLD).tanh();
            let calibrated_noise = calibrated - dry;
            saturated = dry + self.noise_layer_gain as f64 * calibrated_noise;
        }
        let output = flush_denormal((saturated * detail::OUTPUT_TRIM) as f32);

        self.sample_index += 1;
        if self.sample_index >= self.maximum_samples {
            self.active = false;
        }
        output
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.sample_index = 0;
        self.maximum_samples = 0;
        self.core_step = 1.0;
        self.noise_layer_gain = 1.0;
        self.air_spread = 0.0;
        self.air_highpass_coefficient = 0.0;
        self.air_highpass_input = 0.0;
        self.air_highpass_output = 0.0;
        self.upsampler.configure(1.0);
    }
}
