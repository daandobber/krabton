//! The programs, made by a script from the decompiled effects in
//! midiverb_emulator by thement (GPL-3.0): do not edit by hand. Each runs once
//! per sample of the machine: the input, the 16 k of delay memory, where the
//! memory stands now, and the two LFOs of the programs that have them.

use crate::{rd, wr, Dram, Program};


fn midiverb_effect_1(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15992) - 64) / 2 + rd(d, p + (15290) - 131) / 2 + rd(d, p + (14835) - 120) / 2 + rd(d, p + (14696) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15729) - 40) / 2 + rd(d, p + (15555) - 167) / 2 + rd(d, p + (15054) - 151) / 2 + rd(d, p + (14397) - 213) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 113) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 113) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15992) - 261) / 2 + rd(d, p + (15729) - 172) / 2) as i16 as i32;
    wr(d, p + (15729), -acc);
    acc = (-acc / 2 + rd(d, p + (15729) - 172)) as i16 as i32;
    wr(d, p + (15555), acc);
    acc = (rd(d, p + (15555) - 263) / 2 + rd(d, p + (15290) - 234) / 2) as i16 as i32;
    wr(d, p + (15290), -acc);
    acc = (-acc / 2 + rd(d, p + (15290) - 234)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (14107) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (14107), acc);
    acc = (rd(d, p + (14107) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15992), acc);
    acc = (rd(d, p + (15054) - 217) / 2 + rd(d, p + (14835) - 137) / 2) as i16 as i32;
    wr(d, p + (14835), -acc);
    acc = (-acc / 2 + rd(d, p + (14835) - 137)) as i16 as i32;
    wr(d, p + (14696), acc);
    acc = (rd(d, p + (14696) - 297) / 2 + rd(d, p + (14397) - 285) / 2) as i16 as i32;
    wr(d, p + (14397), -acc);
    acc = (-acc / 2 + rd(d, p + (14397) - 285)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (14104) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (14104), acc);
    acc = (rd(d, p + (14104) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15054), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_2(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15992) - 64) / 2 + rd(d, p + (15290) - 131) / 2 + rd(d, p + (14835) - 120) / 2 + rd(d, p + (14696) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15729) - 40) / 2 + rd(d, p + (15555) - 167) / 2 + rd(d, p + (15054) - 151) / 2 + rd(d, p + (14397) - 213) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (14110) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (14110), acc);
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (14110) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 113) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 113) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15992) - 261) / 2 + rd(d, p + (15729) - 172) / 2) as i16 as i32;
    wr(d, p + (15729), -acc);
    acc = (-acc / 2 + rd(d, p + (15729) - 172)) as i16 as i32;
    wr(d, p + (15555), acc);
    acc = (rd(d, p + (15555) - 263) / 2 + rd(d, p + (15290) - 234) / 2) as i16 as i32;
    wr(d, p + (15290), -acc);
    acc = (-acc / 2 + rd(d, p + (15290) - 234)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (14107) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (14107), acc);
    acc = (rd(d, p + (14107) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15992), acc);
    acc = (rd(d, p + (15054) - 217) / 2 + rd(d, p + (14835) - 137) / 2) as i16 as i32;
    wr(d, p + (14835), -acc);
    acc = (-acc / 2 + rd(d, p + (14835) - 137)) as i16 as i32;
    wr(d, p + (14696), acc);
    acc = (rd(d, p + (14696) - 297) / 2 + rd(d, p + (14397) - 285) / 2) as i16 as i32;
    wr(d, p + (14397), -acc);
    acc = (-acc / 2 + rd(d, p + (14397) - 285)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (14104) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (14104), acc);
    acc = (rd(d, p + (14104) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15054), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_3(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15812) - 64) / 2 + rd(d, p + (14975) - 396) / 2 + rd(d, p + (13974) - 339) / 2 + rd(d, p + (13216) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15812) - 303) / 2 + rd(d, p + (14975) - 167) / 2 + rd(d, p + (13974) - 151) / 2 + rd(d, p + (13216) - 512) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15812) - 461) / 2 + rd(d, p + (15349) - 372) / 2) as i16 as i32;
    wr(d, p + (15349), -acc);
    acc = (-acc / 2 + rd(d, p + (15349) - 372)) as i16 as i32;
    wr(d, p + (14975), acc);
    acc = (-rd(d, p + (14975) - 563) / 4 + rd(d, p + (14410) - 434) / 2) as i16 as i32;
    wr(d, p + (14410), -acc);
    acc = (-acc / 2 + rd(d, p + (14410) - 434)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12127) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12127), acc);
    acc = (rd(d, p + (12127) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15812), acc);
    acc = (rd(d, p + (13974) - 417) / 2 + rd(d, p + (13555) - 337) / 2) as i16 as i32;
    wr(d, p + (13555), -acc);
    acc = (-acc / 2 + rd(d, p + (13555) - 337)) as i16 as i32;
    wr(d, p + (13216), acc);
    acc = (-rd(d, p + (13216) - 597) / 4 + rd(d, p + (12617) - 485) / 2) as i16 as i32;
    wr(d, p + (12617), -acc);
    acc = (-acc / 2 + rd(d, p + (12617) - 485)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12124) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12124), acc);
    acc = (rd(d, p + (12124) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13974), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_4(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15992) - 64) / 2 + rd(d, p + (15255) - 396) / 2 + rd(d, p + (14554) - 339) / 2 + rd(d, p + (13796) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15992) - 303) / 2 + rd(d, p + (15255) - 167) / 2 + rd(d, p + (14554) - 151) / 2 + rd(d, p + (13397) - 113) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 103) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 103) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15992) - 461) / 2 + rd(d, p + (15529) - 272) / 2) as i16 as i32;
    wr(d, p + (15529), -acc);
    acc = (-acc / 2 + rd(d, p + (15529) - 272)) as i16 as i32;
    wr(d, p + (15255), acc);
    acc = (rd(d, p + (15255) - 463) / 2 + rd(d, p + (14790) - 234) / 2) as i16 as i32;
    wr(d, p + (14790), -acc);
    acc = (-acc / 2 + rd(d, p + (14790) - 234)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (13007) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (13007), acc);
    acc = (rd(d, p + (13007) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15992), acc);
    acc = (rd(d, p + (14554) - 417) / 2 + rd(d, p + (14135) - 337) / 2) as i16 as i32;
    wr(d, p + (14135), -acc);
    acc = (-acc / 2 + rd(d, p + (14135) - 337)) as i16 as i32;
    wr(d, p + (13796), acc);
    acc = (rd(d, p + (13796) - 397) / 2 + rd(d, p + (13397) - 385) / 2) as i16 as i32;
    wr(d, p + (13397), -acc);
    acc = (-acc / 2 + rd(d, p + (13397) - 385)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (13004) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (13004), acc);
    acc = (rd(d, p + (13004) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14554), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_5(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15978) - 64) / 2 + rd(d, p + (15141) - 196) / 2 + rd(d, p + (14340) - 339) / 2 + rd(d, p + (13582) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15978) - 303) / 2 + rd(d, p + (15141) - 167) / 2 + rd(d, p + (14340) - 151) / 2 + rd(d, p + (13582) - 212) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (12696) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (12696), acc);
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (12696) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 103) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 103) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15978) - 461) / 2 + rd(d, p + (15515) - 372) / 2) as i16 as i32;
    wr(d, p + (15515), -acc);
    acc = (-acc / 2 + rd(d, p + (15515) - 372)) as i16 as i32;
    wr(d, p + (15141), acc);
    acc = (rd(d, p + (15141) - 363) / 2 + rd(d, p + (14776) - 434) / 2) as i16 as i32;
    wr(d, p + (14776), -acc);
    acc = (-acc / 2 + rd(d, p + (14776) - 434)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12693) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12693), acc);
    acc = (rd(d, p + (12693) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15978), acc);
    acc = (rd(d, p + (14340) - 417) / 2 + rd(d, p + (13921) - 337) / 2) as i16 as i32;
    wr(d, p + (13921), -acc);
    acc = (-acc / 2 + rd(d, p + (13921) - 337)) as i16 as i32;
    wr(d, p + (13582), acc);
    acc = (rd(d, p + (13582) - 397) / 2 + rd(d, p + (13183) - 485) / 2) as i16 as i32;
    wr(d, p + (13183), -acc);
    acc = (-acc / 2 + rd(d, p + (13183) - 485)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12690) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12690), acc);
    acc = (rd(d, p + (12690) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14340), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_6(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15812) - 64) / 2 + rd(d, p + (14975) - 396) / 2 + rd(d, p + (13974) - 339) / 2 + rd(d, p + (13216) - 179) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15812) - 303) / 2 + rd(d, p + (14975) - 167) / 2 + rd(d, p + (13974) - 151) / 2 + rd(d, p + (13216) - 512) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15812) - 461) / 2 + rd(d, p + (15349) - 372) / 2) as i16 as i32;
    wr(d, p + (15349), -acc);
    acc = (-acc / 2 + rd(d, p + (15349) - 372)) as i16 as i32;
    wr(d, p + (14975), acc);
    acc = (rd(d, p + (14975) - 563) / 2 + rd(d, p + (14410) - 434) / 2) as i16 as i32;
    wr(d, p + (14410), -acc);
    acc = (-acc / 2 + rd(d, p + (14410) - 434)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12127) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12127), acc);
    acc = (rd(d, p + (12127) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15812), acc);
    acc = (rd(d, p + (13974) - 417) / 2 + rd(d, p + (13555) - 337) / 2) as i16 as i32;
    wr(d, p + (13555), -acc);
    acc = (-acc / 2 + rd(d, p + (13555) - 337)) as i16 as i32;
    wr(d, p + (13216), acc);
    acc = (rd(d, p + (13216) - 597) / 2 + rd(d, p + (12617) - 485) / 2) as i16 as i32;
    wr(d, p + (12617), -acc);
    acc = (-acc / 2 + rd(d, p + (12617) - 485)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12124) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12124), acc);
    acc = (rd(d, p + (12124) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13974), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_7(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (15812) - 64) / 4 + -rd(d, p + (14975) - 396) / 4 + -rd(d, p + (13974) - 339) / 4 + -rd(d, p + (13216) - 179) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (15812) - 303) / 4 + -rd(d, p + (14975) - 167) / 4 + -rd(d, p + (13974) - 151) / 4 + -rd(d, p + (13216) - 512) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (12130) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (12130), acc);
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (12130) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15812) - 461) / 2 + rd(d, p + (15349) - 372) / 2) as i16 as i32;
    wr(d, p + (15349), -acc);
    acc = (-acc / 2 + rd(d, p + (15349) - 372)) as i16 as i32;
    wr(d, p + (14975), acc);
    acc = (rd(d, p + (14975) - 563) / 2 + rd(d, p + (14410) - 434) / 2) as i16 as i32;
    wr(d, p + (14410), -acc);
    acc = (-acc / 2 + rd(d, p + (14410) - 434)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12127) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12127), acc);
    acc = (rd(d, p + (12127) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15812), acc);
    acc = (rd(d, p + (13974) - 417) / 2 + rd(d, p + (13555) - 337) / 2) as i16 as i32;
    wr(d, p + (13555), -acc);
    acc = (-acc / 2 + rd(d, p + (13555) - 337)) as i16 as i32;
    wr(d, p + (13216), acc);
    acc = (rd(d, p + (13216) - 597) / 2 + rd(d, p + (12617) - 485) / 2) as i16 as i32;
    wr(d, p + (12617), -acc);
    acc = (-acc / 2 + rd(d, p + (12617) - 485)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12124) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (12124), acc);
    acc = (rd(d, p + (12124) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13974), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_8(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15645) - 64) / 2 + rd(d, p + (14568) - 496) / 2 + rd(d, p + (11989) - 339) / 2 + rd(d, p + (16230) - 24) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15645) - 403) / 2 + rd(d, p + (14568) - 167) / 2 + rd(d, p + (11989) - 151) / 2 + rd(d, p + (16230) - 357) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15645) - 601) / 2 + rd(d, p + (15042) - 472) / 2) as i16 as i32;
    wr(d, p + (15042), -acc);
    acc = (-acc / 2 + rd(d, p + (15042) - 472)) as i16 as i32;
    wr(d, p + (14568), acc);
    acc = (rd(d, p + (14568) - 683) / 2 + rd(d, p + (13883) - 634) / 2) as i16 as i32;
    wr(d, p + (13883), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (13883) - 634) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (10730) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (10730), acc);
    acc = (rd(d, p + (10730) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15645), acc);
    acc = (rd(d, p + (13247) - 617) / 2 + rd(d, p + (12628) - 637) / 2) as i16 as i32;
    wr(d, p + (12628), -acc);
    acc = (-acc / 2 + rd(d, p + (12628) - 637)) as i16 as i32;
    wr(d, p + (11989), acc);
    acc = (rd(d, p + (11989) - 597) / 2 + rd(d, p + (11390) - 655) / 2) as i16 as i32;
    wr(d, p + (11390), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (11390) - 655) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (10727) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (10727), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (10727) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13247), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_9(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14322) - 123) / 4 + -rd(d, p + (11770) - 987) / 4 + -rd(d, p + (6614) - 678) / 4 + -rd(d, p + (16182) - 142) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14322) - 803) / 4 + -rd(d, p + (11770) - 342) / 4 + -rd(d, p + (6614) - 301) / 4 + -rd(d, p + (15982) - 620) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (3742) - 1) / 2 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (3742), acc);
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (3742) - 1) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) * 5 / 8 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (rd(d, p + (15982) - 0) * 5 / 8 + rd(d, p + (15982) - 367) + rd(d, p + (15113) - 789) / 2) as i16 as i32;
    wr(d, p + (15113), -acc);
    acc = (acc * -27 / 16 + rd(d, p + (15113) - 789) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (-rd(d, p + (14322) - 1203) / 4 + rd(d, p + (13117) - 1345) / 2) as i16 as i32;
    wr(d, p + (13117), -acc);
    acc = (-acc / 2 + rd(d, p + (13117) - 1345)) as i16 as i32;
    wr(d, p + (11770), acc);
    acc = (-rd(d, p + (11770) - 1369) / 4 + rd(d, p + (10399) - 1289) / 2) as i16 as i32;
    wr(d, p + (10399), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (10399) - 1289) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3739) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3739), acc);
    acc = (rd(d, p + (3739) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14322), acc);
    acc = (-rd(d, p + (9108) - 1034) / 4 + rd(d, p + (8072) - 1456) / 2) as i16 as i32;
    wr(d, p + (8072), -acc);
    acc = (-acc / 2 + rd(d, p + (8072) - 1456)) as i16 as i32;
    wr(d, p + (6614), acc);
    acc = (-rd(d, p + (6614) - 1483) / 4 + rd(d, p + (5129) - 1385) / 2) as i16 as i32;
    wr(d, p + (5129), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5129) - 1385) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3736) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3736), acc);
    acc = (rd(d, p + (3736) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (9108), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_10(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14322) - 123) / 4 + -rd(d, p + (11770) - 987) / 4 + -rd(d, p + (6614) - 678) / 4 + -rd(d, p + (16182) - 142) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14322) - 503) / 4 + -rd(d, p + (11770) - 342) / 4 + -rd(d, p + (6614) - 301) / 4 + -rd(d, p + (15982) - 620) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (3742) - 1) * 3 / 4 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (3742), acc);
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (3742) - 1) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) * 5 / 8 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (rd(d, p + (15982) - 0) * 5 / 8 + rd(d, p + (15982) - 367) + rd(d, p + (15113) - 789) / 2) as i16 as i32;
    wr(d, p + (15113), -acc);
    acc = (acc * -27 / 16 + rd(d, p + (15113) - 789) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (-rd(d, p + (14322) - 1203) / 4 + rd(d, p + (13117) - 1345) / 2) as i16 as i32;
    wr(d, p + (13117), -acc);
    acc = (-acc / 2 + rd(d, p + (13117) - 1345)) as i16 as i32;
    wr(d, p + (11770), acc);
    acc = (-rd(d, p + (11770) - 1369) / 4 + rd(d, p + (10399) - 1289) / 2) as i16 as i32;
    wr(d, p + (10399), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (10399) - 1289) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3739) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3739), acc);
    acc = (rd(d, p + (3739) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14322), acc);
    acc = (-rd(d, p + (9108) - 1034) / 4 + rd(d, p + (8072) - 1456) / 2) as i16 as i32;
    wr(d, p + (8072), -acc);
    acc = (-acc / 2 + rd(d, p + (8072) - 1456)) as i16 as i32;
    wr(d, p + (6614), acc);
    acc = (-rd(d, p + (6614) - 1483) / 4 + rd(d, p + (5129) - 1385) / 2) as i16 as i32;
    wr(d, p + (5129), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5129) - 1385) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3736) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3736), acc);
    acc = (rd(d, p + (3736) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (9108), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_11(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (15645) - 64) / 4 + -rd(d, p + (14468) - 496) / 4 + -rd(d, p + (11789) - 339) / 4 + -rd(d, p + (16230) - 24) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (15645) - 403) / 4 + -rd(d, p + (14468) - 167) / 4 + -rd(d, p + (11789) - 151) / 4 + -rd(d, p + (16230) - 357) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15645) - 701) * 5 / 8 + rd(d, p + (14942) - 472) / 2) as i16 as i32;
    wr(d, p + (14942), -acc);
    acc = (-acc / 2 + rd(d, p + (14942) - 472)) as i16 as i32;
    wr(d, p + (14468), acc);
    acc = (rd(d, p + (14468) - 783) * 5 / 8 + rd(d, p + (13683) - 634) / 2) as i16 as i32;
    wr(d, p + (13683), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (13683) - 634) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (10560) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (10560), acc);
    acc = (rd(d, p + (10560) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15645), acc);
    acc = (rd(d, p + (13047) - 617) * 5 / 8 + rd(d, p + (12428) - 637) / 2) as i16 as i32;
    wr(d, p + (12428), -acc);
    acc = (-acc / 2 + rd(d, p + (12428) - 637)) as i16 as i32;
    wr(d, p + (11789), acc);
    acc = (rd(d, p + (11789) - 567) * 5 / 8 + rd(d, p + (11220) - 655) / 2) as i16 as i32;
    wr(d, p + (11220), -acc);
    left = (tmp_5) as i16;
    acc = (acc * -3 / 4 + rd(d, p + (11220) - 655) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (10557) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (10557), acc);
    acc = (rd(d, p + (10557) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13047), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_12(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13241) - 21) / 2 + rd(d, p + (9821) - 1591) / 2 + rd(d, p + (5436) - 1203) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13241) - 801) / 2 + rd(d, p + (9821) - 890) / 2 + rd(d, p + (5436) - 671) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (763) - 1) / 2) as i16 as i32;
    wr(d, p + (763), acc);
    acc = (rd(d, p + (763) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1120) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1120) + rd(d, p + (13633) - 234) / 2) as i16 as i32;
    wr(d, p + (13633), -acc);
    acc = (-acc / 2 + rd(d, p + (13633) - 234) + rd(d, p + (13397) - 154) / 2) as i16 as i32;
    wr(d, p + (13397), -acc);
    acc = (-acc / 2 + rd(d, p + (13397) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (-rd(d, p + (13241) - 1424) / 4 + rd(d, p + (11815) - 1992) / 2) as i16 as i32;
    wr(d, p + (11815), -acc);
    acc = (-acc / 2 + rd(d, p + (11815) - 1992) + rd(d, p + (760) - 1) / 2) as i16 as i32;
    wr(d, p + (760), acc);
    acc = (rd(d, p + (760) - 0)) as i16 as i32;
    wr(d, p + (9821), acc);
    acc = (-rd(d, p + (9821) - 2010) / 4 + rd(d, p + (7809) - 2371) / 2) as i16 as i32;
    wr(d, p + (7809), -acc);
    acc = (-acc / 2 + rd(d, p + (7809) - 2371)) as i16 as i32;
    wr(d, p + (5436), acc);
    acc = (-rd(d, p + (5436) - 2057) / 4 + rd(d, p + (3377) - 2612) / 2) as i16 as i32;
    wr(d, p + (3377), -acc);
    acc = (-acc / 2 + rd(d, p + (3377) - 2612) + tmp_2) as i16 as i32;
    wr(d, p + (13241), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_13(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (15785) - 64) / 4 + -rd(d, p + (14908) - 496) / 4 + -rd(d, p + (12759) - 339) / 4 + -rd(d, p + (16230) - 24) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (15785) - 403) / 4 + -rd(d, p + (14908) - 167) / 4 + -rd(d, p + (12759) - 151) / 4 + -rd(d, p + (16230) - 357) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11503) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (11503), acc);
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (11503) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (-rd(d, p + (16230) - 0) / 4) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (16230) - 0) / 2 + rd(d, p + (16230) - 183) + tmp_1 / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (acc * -27 / 16 + tmp_1 * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15785) - 501) * 3 / 4 + rd(d, p + (15282) - 372) / 2) as i16 as i32;
    wr(d, p + (15282), -acc);
    acc = (-acc / 2 + rd(d, p + (15282) - 372)) as i16 as i32;
    wr(d, p + (14908), acc);
    acc = (rd(d, p + (14908) - 683) * 3 / 4 + rd(d, p + (14223) - 534) / 2) as i16 as i32;
    wr(d, p + (14223), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (14223) - 534) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (11500) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (11500), acc);
    acc = (rd(d, p + (11500) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15785), acc);
    acc = (rd(d, p + (13687) - 417) * 3 / 4 + rd(d, p + (13268) - 507) / 2) as i16 as i32;
    wr(d, p + (13268), -acc);
    acc = (-acc / 2 + rd(d, p + (13268) - 507)) as i16 as i32;
    wr(d, p + (12759), acc);
    acc = (rd(d, p + (12759) - 697) * 3 / 4 + rd(d, p + (12060) - 555) / 2) as i16 as i32;
    wr(d, p + (12060), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (12060) - 555) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (11497) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (11497), acc);
    acc = (rd(d, p + (11497) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13687), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_14(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (14236) - 123) / 2 + rd(d, p + (11884) - 987) / 2 + rd(d, p + (6908) - 678) / 2 + rd(d, p + (16182) - 142) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (14236) - 803) / 2 + rd(d, p + (11884) - 342) / 2 + rd(d, p + (6908) - 301) / 2 + rd(d, p + (15982) - 620) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (3836) - 1) / 2 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (3836), acc);
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (3836) - 1) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) * 5 / 8 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (rd(d, p + (15982) - 0) * 5 / 8 + rd(d, p + (15982) - 367) + rd(d, p + (15113) - 875) / 2) as i16 as i32;
    wr(d, p + (15113), -acc);
    acc = (acc * -27 / 16 + rd(d, p + (15113) - 875) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (14236) - 1203) / 2 + rd(d, p + (13031) - 1145) / 2) as i16 as i32;
    wr(d, p + (13031), -acc);
    acc = (-acc / 2 + rd(d, p + (13031) - 1145)) as i16 as i32;
    wr(d, p + (11884), acc);
    acc = (rd(d, p + (11884) - 1569) / 2 + rd(d, p + (10313) - 1339) / 2) as i16 as i32;
    wr(d, p + (10313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (10313) - 1339) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3833) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3833), acc);
    acc = (rd(d, p + (3833) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14236), acc);
    acc = (rd(d, p + (8972) - 1004) / 2 + rd(d, p + (7966) - 1056) / 2) as i16 as i32;
    wr(d, p + (7966), -acc);
    acc = (-acc / 2 + rd(d, p + (7966) - 1056)) as i16 as i32;
    wr(d, p + (6908), acc);
    acc = (rd(d, p + (6908) - 1683) / 2 + rd(d, p + (5223) - 1385) / 2) as i16 as i32;
    wr(d, p + (5223), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5223) - 1385) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3830) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3830), acc);
    acc = (rd(d, p + (3830) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (8972), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_15(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13855) - 123) / 4 + -rd(d, p + (11303) - 987) / 4 + -rd(d, p + (5447) - 678) / 4 + -rd(d, p + (16182) - 142) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13855) - 803) / 4 + -rd(d, p + (11303) - 342) / 4 + -rd(d, p + (5447) - 301) / 4 + -rd(d, p + (15982) - 620) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) * 5 / 8 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (rd(d, p + (15982) - 0) * 5 / 8 + rd(d, p + (15982) - 367) + rd(d, p + (15113) - 1256) / 2) as i16 as i32;
    wr(d, p + (15113), -acc);
    acc = (acc * -27 / 16 + rd(d, p + (15113) - 1256) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13855) - 1203) + rd(d, p + (12650) - 1345) / 2) as i16 as i32;
    wr(d, p + (12650), -acc);
    acc = (-acc / 2 + rd(d, p + (12650) - 1345)) as i16 as i32;
    wr(d, p + (11303), acc);
    acc = (-rd(d, p + (11303) - 1569) / 4 + rd(d, p + (9732) - 1789) / 2) as i16 as i32;
    wr(d, p + (9732), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (9732) - 1789) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (1672) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1672), acc);
    acc = (rd(d, p + (1672) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13855), acc);
    acc = (rd(d, p + (7941) - 1034) + rd(d, p + (6905) - 1456) / 2) as i16 as i32;
    wr(d, p + (6905), -acc);
    acc = (-acc / 2 + rd(d, p + (6905) - 1456)) as i16 as i32;
    wr(d, p + (5447), acc);
    acc = (-rd(d, p + (5447) - 1983) / 4 + rd(d, p + (3462) - 1785) / 2) as i16 as i32;
    wr(d, p + (3462), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3462) - 1785) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (1669) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1669), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (1669) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (7941), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_16(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14124) - 123) / 4 + -rd(d, p + (11572) - 987) / 4 + -rd(d, p + (5716) - 678) / 4 + -rd(d, p + (16182) - 142) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14124) - 803) / 4 + -rd(d, p + (11572) - 342) / 4 + -rd(d, p + (5716) - 301) / 4 + -rd(d, p + (15982) - 620) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (1944) - 1) / 2 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (1944), acc);
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (1944) - 0) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) * 5 / 8 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (rd(d, p + (15982) - 0) * 5 / 8 + rd(d, p + (15982) - 367) + rd(d, p + (15113) - 987) / 2) as i16 as i32;
    wr(d, p + (15113), -acc);
    acc = (acc * -27 / 16 + rd(d, p + (15113) - 987) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (14124) - 1203) * 3 / 4 + rd(d, p + (12919) - 1345) / 2) as i16 as i32;
    wr(d, p + (12919), -acc);
    acc = (-acc / 2 + rd(d, p + (12919) - 1345)) as i16 as i32;
    wr(d, p + (11572), acc);
    acc = (rd(d, p + (11572) - 1569) / 2 + rd(d, p + (10001) - 1789) / 2) as i16 as i32;
    wr(d, p + (10001), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (10001) - 1789) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (1941) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1941), acc);
    acc = (rd(d, p + (1941) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (14124), acc);
    acc = (rd(d, p + (8210) - 1034) * 3 / 4 + rd(d, p + (7174) - 1456) / 2) as i16 as i32;
    wr(d, p + (7174), -acc);
    acc = (-acc / 2 + rd(d, p + (7174) - 1456)) as i16 as i32;
    wr(d, p + (5716), acc);
    acc = (rd(d, p + (5716) - 1983) / 2 + rd(d, p + (3731) - 1785) / 2) as i16 as i32;
    wr(d, p + (3731), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3731) - 1785) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (1938) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1938), acc);
    acc = (rd(d, p + (1938) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (8210), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_17(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (15084) - 12) / 4 + -rd(d, p + (13237) - 987) / 4 + -rd(d, p + (10675) - 978) / 4 + -rd(d, p + (8381) - 745) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (15084) - 523) / 4 + -rd(d, p + (13237) - 342) / 4 + -rd(d, p + (10675) - 12) / 4 + -rd(d, p + (8381) - 301) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6209) - 1) / 2 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (6209), acc);
    acc = (rd(d, p + (16282) - 78) * 5 / 8 + rd(d, p + (6209) - 1) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16202) - 147) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 78)) as i16 as i32;
    wr(d, p + (16202), -acc);
    acc = (rd(d, p + (16053) - 367) * 5 / 8 + rd(d, p + (16202) - 0) * 5 / 8 + rd(d, p + (16202) - 147)) as i16 as i32;
    wr(d, p + (16053), -acc);
    acc = (rd(d, p + (16053) - 0) * 135 / 64 + rd(d, p + (16053) - 367) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15084) - 798) * 3 / 4 + rd(d, p + (14284) - 1045) / 2) as i16 as i32;
    wr(d, p + (14284), -acc);
    acc = (-acc / 2 + rd(d, p + (14284) - 1045)) as i16 as i32;
    wr(d, p + (13237), acc);
    acc = (rd(d, p + (13237) - 1269) * 3 / 4 + rd(d, p + (11966) - 1289) / 2) as i16 as i32;
    wr(d, p + (11966), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (11966) - 1289) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6206) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6206), acc);
    acc = (rd(d, p + (6206) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15084), acc);
    acc = (rd(d, p + (10675) - 1034) * 3 / 4 + rd(d, p + (9639) - 1256) / 2) as i16 as i32;
    wr(d, p + (9639), -acc);
    acc = (-acc / 2 + rd(d, p + (9639) - 1256)) as i16 as i32;
    wr(d, p + (8381), acc);
    acc = (rd(d, p + (8381) - 1183) * 3 / 4 + rd(d, p + (7196) - 985) / 2) as i16 as i32;
    wr(d, p + (7196), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (7196) - 985) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6203) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6203), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (6203) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (10675), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_18(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (15084) - 12) / 4 + -rd(d, p + (13237) - 987) / 4 + -rd(d, p + (10675) - 978) / 4 + -rd(d, p + (8381) - 745) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (15084) - 523) / 4 + -rd(d, p + (13237) - 342) / 4 + -rd(d, p + (10675) - 12) / 4 + -rd(d, p + (8381) - 301) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16282) - 78) * 5 / 8 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (rd(d, p + (16202) - 147) * 5 / 8 + rd(d, p + (16282) - 0) * 5 / 8 + rd(d, p + (16282) - 78)) as i16 as i32;
    wr(d, p + (16202), -acc);
    acc = (rd(d, p + (16053) - 367) * 5 / 8 + rd(d, p + (16202) - 0) * 5 / 8 + rd(d, p + (16202) - 147)) as i16 as i32;
    wr(d, p + (16053), -acc);
    acc = (rd(d, p + (16053) - 0) * 135 / 64 + rd(d, p + (16053) - 367) * 27 / 8) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15084) - 798) * 3 / 4 + rd(d, p + (14284) - 1045) / 2) as i16 as i32;
    wr(d, p + (14284), -acc);
    acc = (-acc / 2 + rd(d, p + (14284) - 1045)) as i16 as i32;
    wr(d, p + (13237), acc);
    acc = (rd(d, p + (13237) - 1269) * 3 / 4 + rd(d, p + (11966) - 1289) / 2) as i16 as i32;
    wr(d, p + (11966), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (11966) - 1289) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6206) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6206), acc);
    acc = (rd(d, p + (6206) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15084), acc);
    acc = (rd(d, p + (10675) - 1034) * 3 / 4 + rd(d, p + (9639) - 1256) / 2) as i16 as i32;
    wr(d, p + (9639), -acc);
    acc = (-acc / 2 + rd(d, p + (9639) - 1256)) as i16 as i32;
    wr(d, p + (8381), acc);
    acc = (rd(d, p + (8381) - 1183) * 3 / 4 + rd(d, p + (7196) - 985) / 2) as i16 as i32;
    wr(d, p + (7196), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (7196) - 985) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6203) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6203), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (6203) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (10675), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_19(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13793) - 211) / 4 + -rd(d, p + (10473) - 1391) / 4 + -rd(d, p + (6488) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13793) - 1101) / 4 + -rd(d, p + (10473) - 690) / 4 + -rd(d, p + (6488) - 471) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (2415) - 1) / 2) as i16 as i32;
    wr(d, p + (2415), acc);
    acc = (rd(d, p + (2415) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 668) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 668) + rd(d, p + (14955) - 808) / 2) as i16 as i32;
    wr(d, p + (14955), -acc);
    acc = (-acc / 2 + rd(d, p + (14955) - 808) + rd(d, p + (14145) - 204) / 2) as i16 as i32;
    wr(d, p + (14145), -acc);
    acc = (-acc / 2 + rd(d, p + (14145) - 204) + rd(d, p + (13939) - 144) / 2) as i16 as i32;
    wr(d, p + (13939), -acc);
    acc = (-acc / 2 + rd(d, p + (13939) - 144)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13793) - 1724) * 3 / 4 + rd(d, p + (12067) - 1592) / 2) as i16 as i32;
    wr(d, p + (12067), -acc);
    acc = (-acc / 2 + rd(d, p + (12067) - 1592)) as i16 as i32;
    wr(d, p + (10473), acc);
    acc = (rd(d, p + (10473) - 1910) * 5 / 8 + rd(d, p + (8561) - 2071) / 2) as i16 as i32;
    wr(d, p + (8561), -acc);
    acc = (-acc / 2 + rd(d, p + (8561) - 2071)) as i16 as i32;
    wr(d, p + (6488), acc);
    acc = (rd(d, p + (6488) - 1857) * 5 / 8 + rd(d, p + (4629) - 2212) / 2) as i16 as i32;
    wr(d, p + (4629), -acc);
    acc = (rd(d, p + (2412) - 1) / 2) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (-acc / 2 + rd(d, p + (2412) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (2412), acc);
    acc = (rd(d, p + (2412) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13793), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_20(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12941) - 211) / 4 + -rd(d, p + (9021) - 1391) / 4 + -rd(d, p + (4636) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12941) - 1101) / 4 + -rd(d, p + (9021) - 690) / 4 + -rd(d, p + (4636) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (363) - 1) / 2) as i16 as i32;
    wr(d, p + (363), acc);
    acc = (rd(d, p + (363) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1420) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1420) + rd(d, p + (13333) - 234) / 2) as i16 as i32;
    wr(d, p + (13333), -acc);
    acc = (-acc / 2 + rd(d, p + (13333) - 234) + rd(d, p + (13097) - 154) / 2) as i16 as i32;
    wr(d, p + (13097), -acc);
    acc = (-acc / 2 + rd(d, p + (13097) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12941) - 1924) * 3 / 4 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2010) / 2 + rd(d, p + (7009) - 2371) / 2) as i16 as i32;
    wr(d, p + (7009), -acc);
    acc = (-acc / 2 + rd(d, p + (7009) - 2371)) as i16 as i32;
    wr(d, p + (4636), acc);
    acc = (rd(d, p + (4636) - 1657) / 2 + rd(d, p + (2977) - 2612) / 2) as i16 as i32;
    wr(d, p + (2977), -acc);
    acc = (-acc / 2 + rd(d, p + (2977) - 2612)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (360) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (360), acc);
    acc = (rd(d, p + (360) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12941), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_21(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13306) - 21) / 2 + rd(d, p + (10886) - 1391) / 2 + rd(d, p + (7601) - 1503) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (13306) - 876) / 2 + rd(d, p + (10886) - 890) / 2 + rd(d, p + (7601) - 671) / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (tmp_1 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (3728) - 1) / 2) as i16 as i32;
    wr(d, p + (3728), acc);
    acc = (rd(d, p + (3728) - 1) * 81 / 32 + rd(d, p + (16381) - 251) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 251) + rd(d, p + (16128) - 431) / 2) as i16 as i32;
    wr(d, p + (16128), -acc);
    acc = (-acc / 2 + rd(d, p + (16128) - 431) + rd(d, p + (15695) - 768) / 2) as i16 as i32;
    wr(d, p + (15695), -acc);
    acc = (-acc / 2 + rd(d, p + (15695) - 768) + rd(d, p + (14925) - 1320) / 2) as i16 as i32;
    wr(d, p + (14925), -acc);
    acc = (-acc / 2 + rd(d, p + (14925) - 1320) + rd(d, p + (13603) - 189) / 2) as i16 as i32;
    wr(d, p + (13603), -acc);
    acc = (-acc / 2 + rd(d, p + (13603) - 189) + rd(d, p + (13412) - 104) / 2) as i16 as i32;
    wr(d, p + (13412), -acc);
    acc = (-acc / 2 + rd(d, p + (13412) - 104)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13306) - 1124) * 3 / 4 + rd(d, p + (12180) - 1292) / 2) as i16 as i32;
    wr(d, p + (12180), -acc);
    acc = (-acc / 2 + rd(d, p + (12180) - 1292)) as i16 as i32;
    wr(d, p + (10886), acc);
    acc = (rd(d, p + (10886) - 1410) / 2 + rd(d, p + (9474) - 1871) / 2) as i16 as i32;
    wr(d, p + (9474), -acc);
    acc = (-acc / 2 + rd(d, p + (9474) - 1871)) as i16 as i32;
    wr(d, p + (7601), acc);
    acc = (rd(d, p + (7601) - 1657) / 2 + rd(d, p + (5942) - 2212) / 2) as i16 as i32;
    wr(d, p + (5942), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5942) - 2212) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3725) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3725), acc);
    acc = (rd(d, p + (3725) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13306), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_22(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14810) - 11) / 4 + -rd(d, p + (13099) - 796) / 4 + -rd(d, p + (10902) - 1001) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14810) - 11) / 4 + -rd(d, p + (13099) - 445) / 4 + -rd(d, p + (10902) - 336) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (8564) - 1) / 2) as i16 as i32;
    wr(d, p + (8564), acc);
    acc = (rd(d, p + (8564) - 1) * 81 / 32 + rd(d, p + (16381) - 160) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 160) + rd(d, p + (16219) - 215) / 2) as i16 as i32;
    wr(d, p + (16219), -acc);
    acc = (-acc / 2 + rd(d, p + (16219) - 215) + rd(d, p + (16002) - 434) / 2) as i16 as i32;
    wr(d, p + (16002), -acc);
    acc = (-acc / 2 + rd(d, p + (16002) - 434) + rd(d, p + (15566) - 557) / 2) as i16 as i32;
    wr(d, p + (15566), -acc);
    acc = (-acc / 2 + rd(d, p + (15566) - 557) + rd(d, p + (15007) - 117) / 2) as i16 as i32;
    wr(d, p + (15007), -acc);
    acc = (-acc / 2 + rd(d, p + (15007) - 117) + rd(d, p + (14888) - 76) / 2) as i16 as i32;
    wr(d, p + (14888), -acc);
    acc = (-acc / 2 + rd(d, p + (14888) - 76)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (14810) - 712) * 5 / 8 + rd(d, p + (14096) - 995) / 2) as i16 as i32;
    wr(d, p + (14096), -acc);
    acc = (-acc / 2 + rd(d, p + (14096) - 995)) as i16 as i32;
    wr(d, p + (13099), acc);
    acc = (rd(d, p + (13099) - 1005) * 3 / 4 + rd(d, p + (12092) - 1188) / 2) as i16 as i32;
    wr(d, p + (12092), -acc);
    acc = (-acc / 2 + rd(d, p + (12092) - 1188)) as i16 as i32;
    wr(d, p + (10902), acc);
    acc = (rd(d, p + (10902) - 1028) * 3 / 4 + rd(d, p + (9872) - 1306) / 2) as i16 as i32;
    wr(d, p + (9872), -acc);
    acc = (-acc / 2 + rd(d, p + (9872) - 1306)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (8561) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (8561), acc);
    acc = (rd(d, p + (8561) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (14810), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_23(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13409) - 21) / 4 + -rd(d, p + (10389) - 1491) / 4 + -rd(d, p + (6804) - 1703) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13409) - 876) / 4 + -rd(d, p + (10389) - 390) / 4 + -rd(d, p + (6804) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 768) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 768) + rd(d, p + (14855) - 1102) / 2) as i16 as i32;
    wr(d, p + (14855), -acc);
    acc = (-acc / 2 + rd(d, p + (14855) - 1102) + rd(d, p + (13751) - 234) / 2) as i16 as i32;
    wr(d, p + (13751), -acc);
    acc = (-acc / 2 + rd(d, p + (13751) - 234) + rd(d, p + (13515) - 104) / 2) as i16 as i32;
    wr(d, p + (13515), -acc);
    acc = (-acc / 2 + rd(d, p + (13515) - 104)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13409) - 1424) * 3 / 4 + rd(d, p + (11983) - 1592) / 2) as i16 as i32;
    wr(d, p + (11983), -acc);
    acc = (-acc / 2 + rd(d, p + (11983) - 1592)) as i16 as i32;
    wr(d, p + (10389), acc);
    acc = (-rd(d, p + (10389) - 1810) / 4 + rd(d, p + (8577) - 1771) / 2) as i16 as i32;
    wr(d, p + (8577), -acc);
    acc = (-acc / 2 + rd(d, p + (8577) - 1771)) as i16 as i32;
    wr(d, p + (6804), acc);
    acc = (rd(d, p + (6804) - 2257) * 3 / 4 + rd(d, p + (4545) - 2312) / 2) as i16 as i32;
    wr(d, p + (4545), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (4545) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (2228) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (2228), acc);
    acc = (rd(d, p + (2228) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13409), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_24(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12841) - 21) / 4 + -rd(d, p + (9821) - 1491) / 4 + -rd(d, p + (6236) - 1703) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12841) - 876) / 4 + -rd(d, p + (9821) - 890) / 4 + -rd(d, p + (6236) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1420) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1420) + rd(d, p + (13233) - 234) / 2) as i16 as i32;
    wr(d, p + (13233), -acc);
    acc = (-acc / 2 + rd(d, p + (13233) - 234) + rd(d, p + (12997) - 154) / 2) as i16 as i32;
    wr(d, p + (12997), -acc);
    acc = (-acc / 2 + rd(d, p + (12997) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12841) - 1424) * 3 / 4 + rd(d, p + (11415) - 1592) / 2) as i16 as i32;
    wr(d, p + (11415), -acc);
    acc = (-acc / 2 + rd(d, p + (11415) - 1592)) as i16 as i32;
    wr(d, p + (9821), acc);
    acc = (rd(d, p + (9821) - 1810) * 3 / 4 + rd(d, p + (8009) - 1771) / 2) as i16 as i32;
    wr(d, p + (8009), -acc);
    acc = (-acc / 2 + rd(d, p + (8009) - 1771)) as i16 as i32;
    wr(d, p + (6236), acc);
    acc = (rd(d, p + (6236) - 2257) / 2 + rd(d, p + (3977) - 2312) / 2) as i16 as i32;
    wr(d, p + (3977), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3977) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (1660) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1660), acc);
    acc = (rd(d, p + (1660) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12841), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_25(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12941) - 211) / 4 + -rd(d, p + (9221) - 1591) / 4 + -rd(d, p + (4836) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12941) - 1101) / 4 + -rd(d, p + (9221) - 890) / 4 + -rd(d, p + (4836) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (363) - 1) / 2) as i16 as i32;
    wr(d, p + (363), acc);
    acc = (rd(d, p + (363) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1420) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1420) + rd(d, p + (13333) - 234) / 2) as i16 as i32;
    wr(d, p + (13333), -acc);
    acc = (-acc / 2 + rd(d, p + (13333) - 234) + rd(d, p + (13097) - 154) / 2) as i16 as i32;
    wr(d, p + (13097), -acc);
    acc = (-acc / 2 + rd(d, p + (13097) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12941) - 1924) * 3 / 4 + rd(d, p + (11015) - 1792) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1792)) as i16 as i32;
    wr(d, p + (9221), acc);
    acc = (rd(d, p + (9221) - 2010) / 2 + rd(d, p + (7209) - 2371) / 2) as i16 as i32;
    wr(d, p + (7209), -acc);
    acc = (-acc / 2 + rd(d, p + (7209) - 2371)) as i16 as i32;
    wr(d, p + (4836), acc);
    acc = (rd(d, p + (4836) - 1857) / 2 + rd(d, p + (2977) - 2612) / 2) as i16 as i32;
    wr(d, p + (2977), -acc);
    acc = (-acc / 2 + rd(d, p + (2977) - 2612)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (360) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (360), acc);
    acc = (rd(d, p + (360) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12941), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_26(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12941) - 211) / 4 + -rd(d, p + (9221) - 1591) / 4 + -rd(d, p + (4836) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12941) - 1101) / 4 + -rd(d, p + (9221) - 890) / 4 + -rd(d, p + (4836) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1420) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1420) + rd(d, p + (13333) - 234) / 2) as i16 as i32;
    wr(d, p + (13333), -acc);
    acc = (-acc / 2 + rd(d, p + (13333) - 234) + rd(d, p + (13097) - 154) / 2) as i16 as i32;
    wr(d, p + (13097), -acc);
    acc = (-acc / 2 + rd(d, p + (13097) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12941) - 1924) * 3 / 4 + rd(d, p + (11015) - 1792) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1792)) as i16 as i32;
    wr(d, p + (9221), acc);
    acc = (rd(d, p + (9221) - 2010) / 2 + rd(d, p + (7209) - 2371) / 2) as i16 as i32;
    wr(d, p + (7209), -acc);
    acc = (-acc / 2 + rd(d, p + (7209) - 2371)) as i16 as i32;
    wr(d, p + (4836), acc);
    acc = (rd(d, p + (4836) - 1857) / 2 + rd(d, p + (2977) - 2612) / 2) as i16 as i32;
    wr(d, p + (2977), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2977) - 2612) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (360) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (360), acc);
    acc = (rd(d, p + (360) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12941), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_27(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12741) - 21) / 4 + -rd(d, p + (9621) - 1491) / 4 + -rd(d, p + (5536) - 1703) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12741) - 876) / 4 + -rd(d, p + (9621) - 890) / 4 + -rd(d, p + (5536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (963) - 1) / 2) as i16 as i32;
    wr(d, p + (963), acc);
    acc = (rd(d, p + (963) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1520) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1520) + rd(d, p + (13133) - 234) / 2) as i16 as i32;
    wr(d, p + (13133), -acc);
    acc = (-acc / 2 + rd(d, p + (13133) - 234) + rd(d, p + (12897) - 154) / 2) as i16 as i32;
    wr(d, p + (12897), -acc);
    acc = (-acc / 2 + rd(d, p + (12897) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12741) - 1624) * 3 / 4 + rd(d, p + (11115) - 1492) / 2) as i16 as i32;
    wr(d, p + (11115), -acc);
    acc = (-acc / 2 + rd(d, p + (11115) - 1492)) as i16 as i32;
    wr(d, p + (9621), acc);
    acc = (rd(d, p + (9621) - 2110) / 2 + rd(d, p + (7509) - 1971) / 2) as i16 as i32;
    wr(d, p + (7509), -acc);
    acc = (-acc / 2 + rd(d, p + (7509) - 1971)) as i16 as i32;
    wr(d, p + (5536), acc);
    acc = (rd(d, p + (5536) - 2257) / 2 + rd(d, p + (3277) - 2312) / 2) as i16 as i32;
    wr(d, p + (3277), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3277) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (960) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (960), acc);
    acc = (rd(d, p + (960) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12741), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_28(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12941) - 211) / 4 + -rd(d, p + (9721) - 1591) / 4 + -rd(d, p + (5336) - 403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12941) - 801) / 4 + -rd(d, p + (9721) - 890) / 4 + -rd(d, p + (5336) - 1234) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1420) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1420) + rd(d, p + (13333) - 234) / 2) as i16 as i32;
    wr(d, p + (13333), -acc);
    acc = (-acc / 2 + rd(d, p + (13333) - 234) + rd(d, p + (13097) - 154) / 2) as i16 as i32;
    wr(d, p + (13097), -acc);
    acc = (-acc / 2 + rd(d, p + (13097) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12941) - 1424) * 3 / 4 + rd(d, p + (11515) - 1792) / 2) as i16 as i32;
    wr(d, p + (11515), -acc);
    acc = (-acc / 2 + rd(d, p + (11515) - 1792)) as i16 as i32;
    wr(d, p + (9721), acc);
    acc = (rd(d, p + (9721) - 2010) / 2 + rd(d, p + (7709) - 2371) / 2) as i16 as i32;
    wr(d, p + (7709), -acc);
    acc = (-acc / 2 + rd(d, p + (7709) - 2371)) as i16 as i32;
    wr(d, p + (5336), acc);
    acc = (rd(d, p + (5336) - 2357) / 2 + rd(d, p + (2977) - 2612) / 2) as i16 as i32;
    wr(d, p + (2977), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2977) - 2612) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (360) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (360), acc);
    acc = (rd(d, p + (360) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12941), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_29(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9421) - 1891) / 4 + -rd(d, p + (5136) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 789) / 4 + -rd(d, p + (9421) - 890) / 4 + -rd(d, p + (5136) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (663) - 1) / 2) as i16 as i32;
    wr(d, p + (663), acc);
    acc = (rd(d, p + (663) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 15 / 16 + rd(d, p + (11015) - 1592) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1592)) as i16 as i32;
    wr(d, p + (9421), acc);
    acc = (rd(d, p + (9421) - 2010) * 3 / 4 + rd(d, p + (7409) - 2271) / 2) as i16 as i32;
    wr(d, p + (7409), -acc);
    acc = (-acc / 2 + rd(d, p + (7409) - 2271)) as i16 as i32;
    wr(d, p + (5136), acc);
    acc = (rd(d, p + (5136) - 2157) / 2 + rd(d, p + (2977) - 2312) / 2) as i16 as i32;
    wr(d, p + (2977), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2977) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (660) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (660), acc);
    acc = (rd(d, p + (660) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_30(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13241) - 21) / 4 + -rd(d, p + (9821) - 1591) / 4 + -rd(d, p + (5436) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13241) - 801) / 4 + -rd(d, p + (9821) - 890) / 4 + -rd(d, p + (5436) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (763) - 1) / 2) as i16 as i32;
    wr(d, p + (763), acc);
    acc = (rd(d, p + (763) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1120) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1120) + rd(d, p + (13633) - 234) / 2) as i16 as i32;
    wr(d, p + (13633), -acc);
    acc = (-acc / 2 + rd(d, p + (13633) - 234) + rd(d, p + (13397) - 154) / 2) as i16 as i32;
    wr(d, p + (13397), -acc);
    acc = (-acc / 2 + rd(d, p + (13397) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13241) - 1424) * 3 / 4 + rd(d, p + (11815) - 1992) / 2) as i16 as i32;
    wr(d, p + (11815), -acc);
    acc = (-acc / 2 + rd(d, p + (11815) - 1992)) as i16 as i32;
    wr(d, p + (9821), acc);
    acc = (rd(d, p + (9821) - 2010) * 3 / 4 + rd(d, p + (7809) - 2371) / 2) as i16 as i32;
    wr(d, p + (7809), -acc);
    acc = (-acc / 2 + rd(d, p + (7809) - 2371)) as i16 as i32;
    wr(d, p + (5436), acc);
    acc = (rd(d, p + (5436) - 2057) * 5 / 8 + rd(d, p + (3377) - 2612) / 2) as i16 as i32;
    wr(d, p + (3377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3377) - 2612) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (760) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (760), acc);
    acc = (rd(d, p + (760) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13241), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_31(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 15 / 16 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 3 / 4 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) / 2 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_32(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13241) - 21) / 4 + -rd(d, p + (9621) - 1591) / 4 + -rd(d, p + (5036) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13241) - 801) / 4 + -rd(d, p + (9621) - 890) / 4 + -rd(d, p + (5036) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1120) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1120) + rd(d, p + (13633) - 234) / 2) as i16 as i32;
    wr(d, p + (13633), -acc);
    acc = (-acc / 2 + rd(d, p + (13633) - 234) + rd(d, p + (13397) - 154) / 2) as i16 as i32;
    wr(d, p + (13397), -acc);
    acc = (-acc / 2 + rd(d, p + (13397) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13241) - 1624) + rd(d, p + (11615) - 1992) / 2) as i16 as i32;
    wr(d, p + (11615), -acc);
    acc = (-acc / 2 + rd(d, p + (11615) - 1992)) as i16 as i32;
    wr(d, p + (9621), acc);
    acc = (rd(d, p + (9621) - 2110) * 3 / 4 + rd(d, p + (7509) - 2471) / 2) as i16 as i32;
    wr(d, p + (7509), -acc);
    acc = (-acc / 2 + rd(d, p + (7509) - 2471)) as i16 as i32;
    wr(d, p + (5036), acc);
    acc = (rd(d, p + (5036) - 2257) / 2 + rd(d, p + (2777) - 2612) / 2) as i16 as i32;
    wr(d, p + (2777), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2777) - 2612) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (160) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (160), acc);
    acc = (rd(d, p + (160) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13241), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_33(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13241) - 21) / 4 + -rd(d, p + (9821) - 1591) / 4 + -rd(d, p + (5436) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13241) - 801) / 4 + -rd(d, p + (9821) - 890) / 4 + -rd(d, p + (5436) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (763) - 1) / 2) as i16 as i32;
    wr(d, p + (763), acc);
    acc = (rd(d, p + (763) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 868) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 868) + rd(d, p + (14755) - 1120) / 2) as i16 as i32;
    wr(d, p + (14755), -acc);
    acc = (-acc / 2 + rd(d, p + (14755) - 1120) + rd(d, p + (13633) - 234) / 2) as i16 as i32;
    wr(d, p + (13633), -acc);
    acc = (-acc / 2 + rd(d, p + (13633) - 234) + rd(d, p + (13397) - 154) / 2) as i16 as i32;
    wr(d, p + (13397), -acc);
    acc = (-acc / 2 + rd(d, p + (13397) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13241) - 1424) * 5 / 8 + rd(d, p + (11815) - 1992) / 2) as i16 as i32;
    wr(d, p + (11815), -acc);
    acc = (-acc / 2 + rd(d, p + (11815) - 1992)) as i16 as i32;
    wr(d, p + (9821), acc);
    acc = (rd(d, p + (9821) - 2010) * 3 / 4 + rd(d, p + (7809) - 2371) / 2) as i16 as i32;
    wr(d, p + (7809), -acc);
    acc = (-acc / 2 + rd(d, p + (7809) - 2371)) as i16 as i32;
    wr(d, p + (5436), acc);
    acc = (rd(d, p + (5436) - 2057) * 3 / 4 + rd(d, p + (3377) - 2612) / 2) as i16 as i32;
    wr(d, p + (3377), -acc);
    acc = (-acc / 2 + rd(d, p + (3377) - 2612)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (760) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (760), acc);
    acc = (rd(d, p + (760) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13241), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_34(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14377) - 21) / 4 + -rd(d, p + (10748) - 79) / 4 + -rd(d, p + (8945) - 1356) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14377) - 501) / 4 + -rd(d, p + (12060) - 890) / 4 + -rd(d, p + (8945) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 221) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 221) + rd(d, p + (16158) - 301) / 2) as i16 as i32;
    wr(d, p + (16158), -acc);
    acc = (-acc / 2 + rd(d, p + (16158) - 301) + rd(d, p + (15855) - 489) / 2) as i16 as i32;
    wr(d, p + (15855), -acc);
    acc = (-acc / 2 + rd(d, p + (15855) - 489) + rd(d, p + (15364) - 721) / 2) as i16 as i32;
    wr(d, p + (15364), -acc);
    acc = (-acc / 2 + rd(d, p + (15364) - 721) + rd(d, p + (14641) - 171) / 2) as i16 as i32;
    wr(d, p + (14641), -acc);
    acc = (-acc / 2 + rd(d, p + (14641) - 171) + rd(d, p + (14468) - 89) / 2) as i16 as i32;
    wr(d, p + (14468), -acc);
    acc = (-acc / 2 + rd(d, p + (14468) - 89)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (14377) - 621) * 3 / 4 + rd(d, p + (13754) - 1692) / 2) as i16 as i32;
    wr(d, p + (13754), -acc);
    acc = (-acc / 2 + rd(d, p + (13754) - 1692)) as i16 as i32;
    wr(d, p + (12060), acc);
    acc = (rd(d, p + (12060) - 1310) * 15 / 16 + rd(d, p + (10748) - 1801) / 2) as i16 as i32;
    wr(d, p + (10748), -acc);
    acc = (-acc / 2 + rd(d, p + (10748) - 1801)) as i16 as i32;
    wr(d, p + (8945), acc);
    acc = (rd(d, p + (8945) - 1457) * -27 / 32 + rd(d, p + (7486) - 1576) / 2) as i16 as i32;
    wr(d, p + (7486), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (7486) - 1576) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (5905) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (5905), acc);
    acc = (rd(d, p + (5905) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (14377), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_35(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9621) - 1691) / 4 + -rd(d, p + (5836) - 1903) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 801) / 4 + -rd(d, p + (9621) - 890) / 4 + -rd(d, p + (5836) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1424) * 3 / 4 + rd(d, p + (11215) - 1592) / 2) as i16 as i32;
    wr(d, p + (11215), -acc);
    acc = (-acc / 2 + rd(d, p + (11215) - 1592)) as i16 as i32;
    wr(d, p + (9621), acc);
    acc = (rd(d, p + (9621) - 1710) * 3 / 4 + rd(d, p + (7909) - 2071) / 2) as i16 as i32;
    wr(d, p + (7909), -acc);
    acc = (-acc / 2 + rd(d, p + (7909) - 2071)) as i16 as i32;
    wr(d, p + (5836), acc);
    acc = (rd(d, p + (5836) - 1957) * -27 / 32 + rd(d, p + (3877) - 2312) / 2) as i16 as i32;
    wr(d, p + (3877), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (3877) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (1560) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (1560), acc);
    acc = (rd(d, p + (1560) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_36(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) / 2 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_37(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) / 2 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_38(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13441) - 21) / 4 + -rd(d, p + (9821) - 1591) / 4 + -rd(d, p + (5736) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13441) - 801) / 4 + -rd(d, p + (9821) - 890) / 4 + -rd(d, p + (5736) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (763) - 1) / 2) as i16 as i32;
    wr(d, p + (763), acc);
    acc = (rd(d, p + (763) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 768) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 768) + rd(d, p + (14855) - 1020) / 2) as i16 as i32;
    wr(d, p + (14855), -acc);
    acc = (-acc / 2 + rd(d, p + (14855) - 1020) + rd(d, p + (13833) - 234) / 2) as i16 as i32;
    wr(d, p + (13833), -acc);
    acc = (-acc / 2 + rd(d, p + (13833) - 234) + rd(d, p + (13597) - 154) / 2) as i16 as i32;
    wr(d, p + (13597), -acc);
    acc = (-acc / 2 + rd(d, p + (13597) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13441) - 1624) * 3 / 4 + rd(d, p + (11815) - 1992) / 2) as i16 as i32;
    wr(d, p + (11815), -acc);
    acc = (-acc / 2 + rd(d, p + (11815) - 1992)) as i16 as i32;
    wr(d, p + (9821), acc);
    acc = (rd(d, p + (9821) - 1710) * 3 / 4 + rd(d, p + (8109) - 2371) / 2) as i16 as i32;
    wr(d, p + (8109), -acc);
    acc = (-acc / 2 + rd(d, p + (8109) - 2371)) as i16 as i32;
    wr(d, p + (5736), acc);
    acc = (rd(d, p + (5736) - 2357) * -27 / 32 + rd(d, p + (3377) - 2612) / 2) as i16 as i32;
    wr(d, p + (3377), -acc);
    acc = (-acc / 2 + rd(d, p + (3377) - 2612)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (760) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (760), acc);
    acc = (rd(d, p + (760) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13441), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_39(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 801) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * -27 / 32 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * -27 / 32 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * 3 / 4 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_40(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 1703) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 901) / 4 + -rd(d, p + (9021) - 990) / 4 + -rd(d, p + (4536) - 571) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 3 / 4 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 3 / 4 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * -27 / 32 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_41(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * -27 / 32 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * 3 / 4 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (-acc / 2 + rd(d, p + (2377) - 2312)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_42(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 801) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 3 / 4 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * -27 / 32 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_43(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9321) - 1891) / 4 + -rd(d, p + (4836) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9321) - 890) / 4 + -rd(d, p + (4836) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1324) + rd(d, p + (11315) - 1992) / 2) as i16 as i32;
    wr(d, p + (11315), -acc);
    acc = (-acc / 2 + rd(d, p + (11315) - 1992)) as i16 as i32;
    wr(d, p + (9321), acc);
    acc = (rd(d, p + (9321) - 2110) * 15 / 16 + rd(d, p + (7209) - 2371) / 2) as i16 as i32;
    wr(d, p + (7209), -acc);
    acc = (-acc / 2 + rd(d, p + (7209) - 2371)) as i16 as i32;
    wr(d, p + (4836), acc);
    acc = (rd(d, p + (4836) - 2257) * 3 / 4 + rd(d, p + (2577) - 2512) / 2) as i16 as i32;
    wr(d, p + (2577), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2577) - 2512) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_44(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 1703) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 901) / 4 + -rd(d, p + (9021) - 990) / 4 + -rd(d, p + (4536) - 571) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 15 / 16 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * -27 / 32 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_45(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (63) - 1) / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) * 15 / 16 + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * -27 / 32 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (60) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (60), acc);
    acc = (rd(d, p + (60) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_46(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1891) / 4 + -rd(d, p + (4536) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 1010) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4536) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) * -27 / 32 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (63) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (63), acc);
    acc = (rd(d, p + (63) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_47(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12641) - 21) / 4 + -rd(d, p + (9021) - 1591) / 4 + -rd(d, p + (4436) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12641) - 812) / 4 + -rd(d, p + (9021) - 890) / 4 + -rd(d, p + (4436) - 871) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (93) - 1) / 2) as i16 as i32;
    wr(d, p + (93), acc);
    acc = (rd(d, p + (93) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2210) * 15 / 16 + rd(d, p + (6809) - 2371) / 2) as i16 as i32;
    wr(d, p + (6809), -acc);
    acc = (-acc / 2 + rd(d, p + (6809) - 2371)) as i16 as i32;
    wr(d, p + (4436), acc);
    acc = (rd(d, p + (4436) - 2127) * -27 / 32 + rd(d, p + (2307) - 2212) / 2) as i16 as i32;
    wr(d, p + (2307), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2307) - 2212) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (90) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (90), acc);
    acc = (rd(d, p + (90) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_48(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (12665) - 21) / 4 + -rd(d, p + (9345) - 1591) / 4 + -rd(d, p + (5360) - 1403) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (12665) - 1010) / 4 + -rd(d, p + (9345) - 890) / 4 + -rd(d, p + (5360) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (787) - 1) / 2) as i16 as i32;
    wr(d, p + (787), acc);
    acc = (rd(d, p + (787) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 243) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 243) + rd(d, p + (12788) - 121) / 2) as i16 as i32;
    wr(d, p + (12788), -acc);
    acc = (-acc / 2 + rd(d, p + (12788) - 121)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12665) - 1324) + rd(d, p + (11339) - 1992) / 2) as i16 as i32;
    wr(d, p + (11339), -acc);
    acc = (-acc / 2 + rd(d, p + (11339) - 1992)) as i16 as i32;
    wr(d, p + (9345), acc);
    acc = (rd(d, p + (9345) - 1610) * 15 / 16 + rd(d, p + (7733) - 2371) / 2) as i16 as i32;
    wr(d, p + (7733), -acc);
    acc = (-acc / 2 + rd(d, p + (7733) - 2371)) as i16 as i32;
    wr(d, p + (5360), acc);
    acc = (rd(d, p + (5360) - 1857) * 15 / 16 + rd(d, p + (3501) - 2712) / 2) as i16 as i32;
    wr(d, p + (3501), -acc);
    acc = (acc / 4 + -rd(d, p + (3501) - 2712) / 2 + rd(d, p + (784) - 1) / 2) as i16 as i32;
    wr(d, p + (784), acc);
    acc = (rd(d, p + (784) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12665), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_49(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13033) - 21) / 4 + -rd(d, p + (9413) - 1891) / 4 + -rd(d, p + (4928) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13033) - 1010) / 4 + -rd(d, p + (9413) - 890) / 4 + -rd(d, p + (4928) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13033) - 1624) + rd(d, p + (11407) - 1992) / 2) as i16 as i32;
    wr(d, p + (11407), -acc);
    acc = (-acc / 2 + rd(d, p + (11407) - 1992)) as i16 as i32;
    wr(d, p + (9413), acc);
    acc = (rd(d, p + (9413) - 2110) * 15 / 16 + rd(d, p + (7301) - 2371) / 2) as i16 as i32;
    wr(d, p + (7301), -acc);
    acc = (-acc / 2 + rd(d, p + (7301) - 2371)) as i16 as i32;
    wr(d, p + (4928), acc);
    acc = (rd(d, p + (4928) - 2157) * 15 / 16 + rd(d, p + (2769) - 2712) / 2) as i16 as i32;
    wr(d, p + (2769), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2769) - 2712) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (55) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (55), acc);
    acc = (rd(d, p + (55) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13033), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_50(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13033) - 21) / 4 + -rd(d, p + (9413) - 1891) / 4 + -rd(d, p + (4928) - 2003) / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (13033) - 1010) / 4 + -rd(d, p + (9413) - 890) / 4 + -rd(d, p + (4928) - 671) / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (55) - 1) / 2) as i16 as i32;
    wr(d, p + (55), acc);
    acc = (rd(d, p + (55) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (13033) - 1624) + rd(d, p + (11407) - 1992) / 2) as i16 as i32;
    wr(d, p + (11407), -acc);
    acc = (-acc / 2 + rd(d, p + (11407) - 1992)) as i16 as i32;
    wr(d, p + (9413), acc);
    acc = (rd(d, p + (9413) - 2110) * 15 / 16 + rd(d, p + (7301) - 2371) / 2) as i16 as i32;
    wr(d, p + (7301), -acc);
    acc = (-acc / 2 + rd(d, p + (7301) - 2371)) as i16 as i32;
    wr(d, p + (4928), acc);
    acc = (rd(d, p + (4928) - 2157) * 15 / 16 + rd(d, p + (2769) - 2712) / 2) as i16 as i32;
    wr(d, p + (2769), -acc);
    acc = (acc / 4 + -rd(d, p + (2769) - 2712) / 2 + rd(d, p + (52) - 1) / 2) as i16 as i32;
    wr(d, p + (52), acc);
    acc = (rd(d, p + (52) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (13033), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_51(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_52(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_53(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_54(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_55(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 6159) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2 + rd(d, p + (0) - 6534) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7039) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_56(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 6159) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2 + rd(d, p + (0) - 6534) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7039) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7221) / 2 + rd(d, p + (0) - 7439) / 2 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 8034) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_57(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 6159) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2 + rd(d, p + (0) - 6534) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7039) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7221) / 2 + rd(d, p + (0) - 7439) / 2 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 8034) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2 + rd(d, p + (0) - 8447) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 8700) / 2 + rd(d, p + (0) - 8885) / 2 + rd(d, p + (0) - 9012) / 2 + rd(d, p + (0) - 9171) / 2 + rd(d, p + (0) - 9215) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_58(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 6159) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2 + rd(d, p + (0) - 6534) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7039) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7221) / 2 + rd(d, p + (0) - 7439) / 2 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 8034) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2 + rd(d, p + (0) - 8447) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 8700) / 2 + rd(d, p + (0) - 8885) / 2 + rd(d, p + (0) - 9012) / 2 + rd(d, p + (0) - 9171) / 2 + rd(d, p + (0) - 9215) / 2 + rd(d, p + (0) - 9449) / 2 + rd(d, p + (0) - 9513) / 2 + rd(d, p + (0) - 9765) / 2 + rd(d, p + (0) - 9867) / 2 + rd(d, p + (0) - 10046) / 2 + rd(d, p + (0) - 10110) / 2 + rd(d, p + (0) - 10285) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 10578) / 2 + rd(d, p + (0) - 10734) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11023) / 2 + rd(d, p + (0) - 11185) / 2 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midiverb_effect_59(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (0) - 1065) / 2 + rd(d, p + (0) - 1119) / 2 + rd(d, p + (0) - 1233) / 2 + rd(d, p + (0) - 1434) / 2 + rd(d, p + (0) - 1555) / 2 + rd(d, p + (0) - 1742) / 2 + rd(d, p + (0) - 1862) / 2 + rd(d, p + (0) - 2046) / 2 + rd(d, p + (0) - 2149) / 2 + rd(d, p + (0) - 2256) / 2 + rd(d, p + (0) - 2443) / 2 + rd(d, p + (0) - 2579) / 2 + rd(d, p + (0) - 2721) / 2 + rd(d, p + (0) - 2873) / 2 + rd(d, p + (0) - 3036) / 2 + rd(d, p + (0) - 3121) / 2 + rd(d, p + (0) - 3267) / 2 + rd(d, p + (0) - 3451) / 2 + rd(d, p + (0) - 3569) / 2 + rd(d, p + (0) - 3725) / 2 + rd(d, p + (0) - 3889) / 2 + rd(d, p + (0) - 4034) / 2 + rd(d, p + (0) - 4152) / 2 + rd(d, p + (0) - 4257) / 2 + rd(d, p + (0) - 4492) / 2 + rd(d, p + (0) - 4556) / 2 + rd(d, p + (0) - 4727) / 2 + rd(d, p + (0) - 4849) / 2 + rd(d, p + (0) - 5034) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 5235) / 2 + rd(d, p + (0) - 5421) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5837) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 6159) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2 + rd(d, p + (0) - 6534) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7039) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7221) / 2 + rd(d, p + (0) - 7439) / 2 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 8034) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2 + rd(d, p + (0) - 8447) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 8700) / 2 + rd(d, p + (0) - 8885) / 2 + rd(d, p + (0) - 9012) / 2 + rd(d, p + (0) - 9171) / 2 + rd(d, p + (0) - 9215) / 2 + rd(d, p + (0) - 9449) / 2 + rd(d, p + (0) - 9513) / 2 + rd(d, p + (0) - 9765) / 2 + rd(d, p + (0) - 9867) / 2 + rd(d, p + (0) - 10046) / 2 + rd(d, p + (0) - 10110) / 2 + rd(d, p + (0) - 10285) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 10578) / 2 + rd(d, p + (0) - 10734) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11023) / 2 + rd(d, p + (0) - 11185) / 2 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 11715) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 12074) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb_effect_60(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1862) / 32 + -rd(d, p + (0) - 1065) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 1742) / 16 + -rd(d, p + (0) - 1233) / 16 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2443) / 16 + -rd(d, p + (0) - 3121) / 16 + -rd(d, p + (0) - 2721) / 16 + rd(d, p + (0) - 2256) / 8 + rd(d, p + (0) - 3036) / 8 + rd(d, p + (0) - 2579) / 8 + rd(d, p + (0) - 3267) / 8 + rd(d, p + (0) - 3451) / 8 + rd(d, p + (0) - 3569) / 8 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 3889) / 8 + rd(d, p + (0) - 4034) / 8 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4849) / 8 + -rd(d, p + (0) - 3725) / 4 + -rd(d, p + (0) - 4556) / 4 + -rd(d, p + (0) - 4727) / 4 + -rd(d, p + (0) - 4257) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 5837) / 4 + -rd(d, p + (0) - 5235) / 4 + -rd(d, p + (0) - 5421) / 4 + -rd(d, p + (0) - 6159) / 4 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_61(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 9375)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 9375)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 2721) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 2256) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 1065) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 3036) / 16 + -rd(d, p + (0) - 4152) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 4492) / 16 + -rd(d, p + (0) - 3725) / 16 + rd(d, p + (0) - 3889) / 8 + rd(d, p + (0) - 4034) / 8 + rd(d, p + (0) - 3121) / 8 + rd(d, p + (0) - 4257) / 8 + rd(d, p + (0) - 3569) / 8 + rd(d, p + (0) - 4556) / 8 + rd(d, p + (0) - 4727) / 8 + rd(d, p + (0) - 4849) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5129) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6159) / 8 + -rd(d, p + (0) - 5759) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 6035) / 4 + -rd(d, p + (0) - 5534) / 4 + -rd(d, p + (0) - 6223) / 4 + -rd(d, p + (0) - 6446) / 4 + -rd(d, p + (0) - 6534) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 6859) / 4 + -rd(d, p + (0) - 7039) / 4 + -rd(d, p + (0) - 7736) / 4 + -rd(d, p + (0) - 7221) / 4 + -rd(d, p + (0) - 7439) / 4 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2 + rd(d, p + (0) - 8447) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 8700) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb_effect_62(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 11718)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 11718)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 2579) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 3036) / 32 + rd(d, p + (0) - 2149) / 32 + -rd(d, p + (0) - 2256) / 16 + -rd(d, p + (0) - 2443) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 2721) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 3121) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 3569) / 16 + -rd(d, p + (0) - 5034) / 16 + -rd(d, p + (0) - 3889) / 16 + -rd(d, p + (0) - 4034) / 16 + -rd(d, p + (0) - 4727) / 16 + -rd(d, p + (0) - 4257) / 16 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 4556) / 8 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4849) / 8 + rd(d, p + (0) - 3725) / 8 + rd(d, p + (0) - 5129) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 5534) / 8 + rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 6035) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 6223) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 6534) / 8 + -rd(d, p + (0) - 6782) / 4 + -rd(d, p + (0) - 5837) / 4 + -rd(d, p + (0) - 7039) / 4 + -rd(d, p + (0) - 6446) / 4 + -rd(d, p + (0) - 7221) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 7534) / 4 + -rd(d, p + (0) - 7736) / 4 + -rd(d, p + (0) - 7891) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9215) / 4 + -rd(d, p + (0) - 8278) / 4 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 8565) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 8885) / 4 + rd(d, p + (0) - 9012) / 2 + rd(d, p + (0) - 9171) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 9449) / 2 + rd(d, p + (0) - 8700) / 2 + rd(d, p + (0) - 9765) / 2 + rd(d, p + (0) - 9867) / 2 + rd(d, p + (0) - 10046) / 2 + rd(d, p + (0) - 10110) / 2 + rd(d, p + (0) - 10285) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 10578) / 2 + rd(d, p + (0) - 10734) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11023) / 2 + rd(d, p + (0) - 11185) / 2 + rd(d, p + (0) - 11254) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb_effect_63(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (acc * 3 / 32 + rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 1555) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 2046) / 32 + rd(d, p + (0) - 3569) / 32 + rd(d, p + (0) - 2256) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 3121) / 32 + rd(d, p + (0) - 2721) / 32 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 3036) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 3725) / 16 + -rd(d, p + (0) - 3889) / 16 + -rd(d, p + (0) - 4034) / 16 + -rd(d, p + (0) - 4152) / 16 + -rd(d, p + (0) - 4257) / 16 + -rd(d, p + (0) - 4492) / 16 + -rd(d, p + (0) - 4556) / 16 + -rd(d, p + (0) - 4727) / 16 + -rd(d, p + (0) - 4849) / 16 + -rd(d, p + (0) - 6035) / 16 + -rd(d, p + (0) - 5129) / 16 + -rd(d, p + (0) - 5235) / 16 + -rd(d, p + (0) - 6223) / 16 + -rd(d, p + (0) - 5534) / 16 + rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5034) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6446) / 8 + rd(d, p + (0) - 6534) / 8 + rd(d, p + (0) - 6782) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 7039) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 7221) / 8 + rd(d, p + (0) - 8565) / 8 + rd(d, p + (0) - 7534) / 8 + rd(d, p + (0) - 7736) / 8 + rd(d, p + (0) - 7891) / 8 + rd(d, p + (0) - 9215) / 8 + rd(d, p + (0) - 8146) / 8 + rd(d, p + (0) - 8278) / 8 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 8700) / 4 + -rd(d, p + (0) - 8885) / 4 + -rd(d, p + (0) - 9012) / 4 + -rd(d, p + (0) - 9171) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9449) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 9765) / 4 + -rd(d, p + (0) - 9867) / 4 + -rd(d, p + (0) - 10046) / 4 + -rd(d, p + (0) - 10110) / 4 + -rd(d, p + (0) - 10285) / 4 + -rd(d, p + (0) - 12074) / 4 + -rd(d, p + (0) - 10578) / 4 + -rd(d, p + (0) - 10734) / 4 + -rd(d, p + (0) - 11715) / 4 + -rd(d, p + (0) - 11023) / 4 + -rd(d, p + (0) - 11185) / 4 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2 + rd(d, p + (0) - 13217) / 2 + rd(d, p + (0) - 13378) / 2 + rd(d, p + (0) - 13534) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb_effect_64(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    left = (rd(d, p + (140) - 135)) as i16;
    right = (rd(d, p + (140) - 137)) as i16;
    acc = (0) as i16 as i32;
    wr(d, p + (140), -acc);
    (left, right)
}

pub static MODEL_1: [Program; 64] = [
    Program { name: ".2 Sec SMALL BRIGHT", run: midiverb_effect_1 },
    Program { name: ".2 Sec SMALL WARM", run: midiverb_effect_2 },
    Program { name: ".2 Sec MEDIUM BRIGHT", run: midiverb_effect_3 },
    Program { name: ".3 Sec SMALL BRIGHT", run: midiverb_effect_4 },
    Program { name: ".3 Sec SMALL WARM", run: midiverb_effect_5 },
    Program { name: ".4 Sec MEDIUM BRIGHT", run: midiverb_effect_6 },
    Program { name: ".4 Sec MEDIUM WARM", run: midiverb_effect_7 },
    Program { name: ".6 Sec SMALL BRIGHT", run: midiverb_effect_8 },
    Program { name: ".6 Sec MEDIUM WARM", run: midiverb_effect_9 },
    Program { name: ".6 Sec MEDIUM DARK", run: midiverb_effect_10 },
    Program { name: ".8 Sec SMALL BRIGHT", run: midiverb_effect_11 },
    Program { name: ".8 Sec LARGE WARM", run: midiverb_effect_12 },
    Program { name: "1.0 Sec SMALL WARM", run: midiverb_effect_13 },
    Program { name: "1.0 Sec MEDIUM WARM", run: midiverb_effect_14 },
    Program { name: "1.0 Sec LARGE BRIGHT", run: midiverb_effect_15 },
    Program { name: "1.2 Sec MEDIUM WARM", run: midiverb_effect_16 },
    Program { name: "1.2 Sec SMALL WARM", run: midiverb_effect_17 },
    Program { name: "1.2 Sec SMALL BRIGHT", run: midiverb_effect_18 },
    Program { name: "1.4 Sec LARGE WARM", run: midiverb_effect_19 },
    Program { name: "1.4 Sec LARGE DARK", run: midiverb_effect_20 },
    Program { name: "1.4 Sec MEDIUM WARM", run: midiverb_effect_21 },
    Program { name: "1.6 Sec SMALL DARK", run: midiverb_effect_22 },
    Program { name: "1.6 Sec LARGE BRIGHT", run: midiverb_effect_23 },
    Program { name: "1.6 Sec MEDIUM BRIGHT", run: midiverb_effect_24 },
    Program { name: "1.8 Sec LARGE DARK", run: midiverb_effect_25 },
    Program { name: "1.8 Sec LARGE BRIGHT", run: midiverb_effect_26 },
    Program { name: "1.8 Sec MEDIUM WARM", run: midiverb_effect_27 },
    Program { name: "2.0 Sec LARGE BRIGHT", run: midiverb_effect_28 },
    Program { name: "2.0 Sec MEDIUM WARM", run: midiverb_effect_29 },
    Program { name: "2.0 Sec LARGE WARM", run: midiverb_effect_30 },
    Program { name: "2.5 Sec MEDIUM WARM", run: midiverb_effect_31 },
    Program { name: "2.5 Sec LARGE BRIGHT", run: midiverb_effect_32 },
    Program { name: "2.5 Sec MEDIUM DARK", run: midiverb_effect_33 },
    Program { name: "2.8 Sec SMALL BRIGHT", run: midiverb_effect_34 },
    Program { name: "2.8 Sec MEDIUM BRIGHT", run: midiverb_effect_35 },
    Program { name: "3.0 Sec LARGE BRIGHT", run: midiverb_effect_36 },
    Program { name: "3.0 Sec LARGE WARM", run: midiverb_effect_37 },
    Program { name: "3.0 Sec MEDIUM DARK", run: midiverb_effect_38 },
    Program { name: "3.5 Sec LARGE BRIGHT", run: midiverb_effect_39 },
    Program { name: "3.5 Sec MEDIUM WARM", run: midiverb_effect_40 },
    Program { name: "4.0 Sec LARGE DARK", run: midiverb_effect_41 },
    Program { name: "4.0 Sec MEDIUM BRIGHT", run: midiverb_effect_42 },
    Program { name: "5 Sec LARGE WARM", run: midiverb_effect_43 },
    Program { name: "8 Sec LARGE BRIGHT", run: midiverb_effect_44 },
    Program { name: "8 Sec LARGE WARM", run: midiverb_effect_45 },
    Program { name: "10 Sec LARGE BRIGHT", run: midiverb_effect_46 },
    Program { name: "10 Sec LARGE WARM", run: midiverb_effect_47 },
    Program { name: "16 Sec LARGE DARK", run: midiverb_effect_48 },
    Program { name: "18 Sec EX. LARGE BRIGHT", run: midiverb_effect_49 },
    Program { name: "20 Sec EX. LARGE DARK", run: midiverb_effect_50 },
    Program { name: "Gated 100 MS", run: midiverb_effect_51 },
    Program { name: "Gated 150 MS", run: midiverb_effect_52 },
    Program { name: "Gated 200 MS", run: midiverb_effect_53 },
    Program { name: "Gated 250 MS", run: midiverb_effect_54 },
    Program { name: "Gated 300 MS", run: midiverb_effect_55 },
    Program { name: "Gated 350 MS", run: midiverb_effect_56 },
    Program { name: "Gated 400 MS", run: midiverb_effect_57 },
    Program { name: "Gated 500 MS", run: midiverb_effect_58 },
    Program { name: "Gated 600 MS", run: midiverb_effect_59 },
    Program { name: "REVERSE - 300 MS", run: midiverb_effect_60 },
    Program { name: "REVERSE - 400 MS", run: midiverb_effect_61 },
    Program { name: "REVERSE - 500 MS", run: midiverb_effect_62 },
    Program { name: "REVERSE - 600 MS", run: midiverb_effect_63 },
    Program { name: "DEFEAT", run: midiverb_effect_64 },
];

fn midifex_effect_1(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 15714) + rd(d, p + (668) - 49) / 2) as i16 as i32;
    wr(d, p + (668), -acc);
    acc = (-acc / 2 + rd(d, p + (668) - 49) + rd(d, p + (617) - 114) / 2) as i16 as i32;
    wr(d, p + (617), -acc);
    acc = (-acc / 2 + rd(d, p + (617) - 114) + rd(d, p + (501) - 178) / 2) as i16 as i32;
    wr(d, p + (501), -acc);
    acc = (-acc / 2 + rd(d, p + (501) - 178)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 15714) + rd(d, p + (321) - 41) / 2) as i16 as i32;
    wr(d, p + (321), -acc);
    acc = (-acc / 2 + rd(d, p + (321) - 41) + rd(d, p + (278) - 195) / 2) as i16 as i32;
    wr(d, p + (278), -acc);
    acc = (-acc / 2 + rd(d, p + (278) - 195) + rd(d, p + (81) - 63) / 2) as i16 as i32;
    wr(d, p + (81), -acc);
    acc = (-acc / 2 + rd(d, p + (81) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_2(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_d = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 11257) + rd(d, p + (5125) - 87) / 2) as i16 as i32;
    wr(d, p + (5125), -acc);
    acc = (-acc / 2 + rd(d, p + (5125) - 87) + rd(d, p + (5036) - 103) / 2) as i16 as i32;
    wr(d, p + (5036), -acc);
    acc = (-acc / 2 + rd(d, p + (5036) - 103) + rd(d, p + (4931) - 127) / 2) as i16 as i32;
    wr(d, p + (4931), -acc);
    acc = (-acc / 2 + rd(d, p + (4931) - 127) + rd(d, p + (4802) - 143) / 2) as i16 as i32;
    wr(d, p + (4802), -acc);
    acc = (-acc / 2 + rd(d, p + (4802) - 143) + rd(d, p + (4657) - 174) / 2) as i16 as i32;
    wr(d, p + (4657), -acc);
    acc = (-acc / 2 + rd(d, p + (4657) - 174) + rd(d, p + (4481) - 215) / 2) as i16 as i32;
    wr(d, p + (4481), -acc);
    acc = (-acc / 2 + rd(d, p + (4481) - 215) + rd(d, p + (4264) - 251) / 2) as i16 as i32;
    wr(d, p + (4264), -acc);
    acc = (-acc / 2 + rd(d, p + (4264) - 251) + rd(d, p + (4011) - 300) / 2) as i16 as i32;
    wr(d, p + (4011), -acc);
    acc = (-acc / 2 + rd(d, p + (4011) - 300) + rd(d, p + (3709) - 369) / 2) as i16 as i32;
    wr(d, p + (3709), -acc);
    acc = (-acc / 2 + rd(d, p + (3709) - 369) + rd(d, p + (3338) - 4) / 2) as i16 as i32;
    wr(d, p + (3338), -acc);
    acc = (-acc / 2 + rd(d, p + (3338) - 4) + rd(d, p + (3332) - 5) / 2) as i16 as i32;
    wr(d, p + (3332), -acc);
    acc = (-acc / 2 + rd(d, p + (3332) - 5) + rd(d, p + (3325) - 62) / 2) as i16 as i32;
    wr(d, p + (3325), -acc);
    acc = (-acc / 2 + rd(d, p + (3325) - 62) + rd(d, p + (3261) - 75) / 2) as i16 as i32;
    wr(d, p + (3261), -acc);
    acc = (-acc / 2 + rd(d, p + (3261) - 75) + rd(d, p + (3184) - 9) / 2) as i16 as i32;
    wr(d, p + (3184), -acc);
    acc = (-acc / 2 + rd(d, p + (3184) - 9)) as i16 as i32;
    tmp_d = (acc) as i16 as i32;
    acc = (tmp_d + rd(d, p + (3173) - 36) / 2) as i16 as i32;
    wr(d, p + (3173), -acc);
    acc = (-acc / 2 + rd(d, p + (3173) - 36)) as i16 as i32;
    wr(d, p + (6), -acc);
    acc = (tmp_d + rd(d, p + (3135) - 30) / 2) as i16 as i32;
    wr(d, p + (3135), -acc);
    acc = (-acc / 2 + rd(d, p + (3135) - 30)) as i16 as i32;
    wr(d, p + (4), -acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_3(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (6080) - 1)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (6080) - 1)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (6083) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 10299) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (6083), acc);
    acc = (rd(d, p + (6083) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 10299) + tmp_1) as i16 as i32;
    wr(d, p + (6074), acc);
    acc = (rd(d, p + (6077) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6074) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (6077), acc);
    acc = (rd(d, p + (6077) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6074) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (6080), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_4(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1cd0 = 0i32;
    let mut tmp_1 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (7379) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 9003) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (7379), acc);
    acc = (rd(d, p + (7379) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 9003) + tmp_1) as i16 as i32;
    wr(d, p + (7355), acc);
    acc = (rd(d, p + (7373) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (7355) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (7373), acc);
    acc = (rd(d, p + (7373) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (7355) - 1) + tmp_1) as i16 as i32;
    tmp_1cd0 = (acc) as i16 as i32;
    acc = (tmp_1cd0) as i16 as i32;
    wr(d, p + (4), acc);
    acc = (tmp_1cd0) as i16 as i32;
    wr(d, p + (201), acc);
    left = (rd(d, p + (201) - 196)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_5(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 8424) + rd(d, p + (7608) - 41) / 2) as i16 as i32;
    wr(d, p + (7608), -acc);
    acc = (-acc / 2 + rd(d, p + (7608) - 41) + rd(d, p + (7565) - 195) / 2) as i16 as i32;
    wr(d, p + (7565), -acc);
    acc = (-acc / 2 + rd(d, p + (7565) - 195) + rd(d, p + (7368) - 63) / 2) as i16 as i32;
    wr(d, p + (7368), -acc);
    acc = (-acc / 2 + rd(d, p + (7368) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 8424) + rd(d, p + (7955) - 49) / 2) as i16 as i32;
    wr(d, p + (7955), -acc);
    acc = (-acc / 2 + rd(d, p + (7955) - 49) + rd(d, p + (7904) - 114) / 2) as i16 as i32;
    wr(d, p + (7904), -acc);
    acc = (-acc / 2 + rd(d, p + (7904) - 114) + rd(d, p + (7788) - 178) / 2) as i16 as i32;
    wr(d, p + (7788), -acc);
    acc = (-acc / 2 + rd(d, p + (7788) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (7303) - 1) / 2) as i16 as i32;
    wr(d, p + (7303), acc);
    acc = (rd(d, p + (7300) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (7303) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (7300), acc);
    acc = (rd(d, p + (7300) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (7303) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_6(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 7722) * 3 / 2 + rd(d, p + (8657) - 15) / 2) as i16 as i32;
    wr(d, p + (8657), -acc);
    acc = (-acc / 2 + rd(d, p + (8657) - 15) + rd(d, p + (8640) - 22) / 2) as i16 as i32;
    wr(d, p + (8640), -acc);
    acc = (-acc / 2 + rd(d, p + (8640) - 22) + rd(d, p + (8616) - 29) / 2) as i16 as i32;
    wr(d, p + (8616), -acc);
    acc = (-acc / 2 + rd(d, p + (8616) - 29) + rd(d, p + (8585) - 45) / 2) as i16 as i32;
    wr(d, p + (8585), -acc);
    acc = (-acc / 2 + rd(d, p + (8585) - 45) + rd(d, p + (8538) - 61) / 2) as i16 as i32;
    wr(d, p + (8538), -acc);
    acc = (-acc / 2 + rd(d, p + (8538) - 61) + rd(d, p + (7565) - 105) / 2) as i16 as i32;
    wr(d, p + (7565), -acc);
    acc = (-acc / 2 + rd(d, p + (7565) - 105) + rd(d, p + (7458) - 149) / 2) as i16 as i32;
    wr(d, p + (7458), -acc);
    acc = (-acc / 2 + rd(d, p + (7458) - 149) + rd(d, p + (7307) - 213) / 2) as i16 as i32;
    wr(d, p + (7307), -acc);
    acc = (-acc / 2 + rd(d, p + (7307) - 213) + rd(d, p + (7092) - 258) / 2) as i16 as i32;
    wr(d, p + (7092), -acc);
    acc = (-acc / 2 + rd(d, p + (7092) - 258)) as i16 as i32;
    wr(d, p + (4), acc);
    acc = (rd(d, p + (16381) - 7722) * 3 / 2 + rd(d, p + (7778) - 17) / 2) as i16 as i32;
    wr(d, p + (7778), -acc);
    acc = (-acc / 2 + rd(d, p + (7778) - 17) + rd(d, p + (7759) - 25) / 2) as i16 as i32;
    wr(d, p + (7759), -acc);
    acc = (-acc / 2 + rd(d, p + (7759) - 25) + rd(d, p + (7732) - 35) / 2) as i16 as i32;
    wr(d, p + (7732), -acc);
    acc = (-acc / 2 + rd(d, p + (7732) - 35) + rd(d, p + (7695) - 52) / 2) as i16 as i32;
    wr(d, p + (7695), -acc);
    acc = (-acc / 2 + rd(d, p + (7695) - 52) + rd(d, p + (7641) - 74) / 2) as i16 as i32;
    wr(d, p + (7641), -acc);
    acc = (-acc / 2 + rd(d, p + (7641) - 74) + rd(d, p + (8475) - 88) / 2) as i16 as i32;
    wr(d, p + (8475), -acc);
    acc = (-acc / 2 + rd(d, p + (8475) - 88) + rd(d, p + (8385) - 123) / 2) as i16 as i32;
    wr(d, p + (8385), -acc);
    acc = (-acc / 2 + rd(d, p + (8385) - 123) + rd(d, p + (8260) - 174) / 2) as i16 as i32;
    wr(d, p + (8260), -acc);
    acc = (-acc / 2 + rd(d, p + (8260) - 174) + rd(d, p + (8084) - 304) / 2) as i16 as i32;
    wr(d, p + (8084), -acc);
    acc = (-acc / 2 + rd(d, p + (8084) - 304)) as i16 as i32;
    wr(d, p + (601), acc);
    acc = (rd(d, p + (6832) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (6832), acc);
    left = (rd(d, p + (601) - 596)) as i16;
    acc = (rd(d, p + (6826) - 1) * 5 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6832) - 1) * 3 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (6826), acc);
    acc = (rd(d, p + (6826) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_7(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7385) + rd(d, p + (8997) - 49) / 2) as i16 as i32;
    wr(d, p + (8997), -acc);
    acc = (-acc / 2 + rd(d, p + (8997) - 49) + rd(d, p + (8946) - 114) / 2) as i16 as i32;
    wr(d, p + (8946), -acc);
    acc = (-acc / 2 + rd(d, p + (8946) - 114) + rd(d, p + (8830) - 178) / 2) as i16 as i32;
    wr(d, p + (8830), -acc);
    acc = (-acc / 2 + rd(d, p + (8830) - 178)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 7385) + rd(d, p + (8650) - 41) / 2) as i16 as i32;
    wr(d, p + (8650), -acc);
    acc = (-acc / 2 + rd(d, p + (8650) - 41) + rd(d, p + (8607) - 195) / 2) as i16 as i32;
    wr(d, p + (8607), -acc);
    acc = (-acc / 2 + rd(d, p + (8607) - 195) + rd(d, p + (8410) - 63) / 2) as i16 as i32;
    wr(d, p + (8410), -acc);
    acc = (-acc / 2 + rd(d, p + (8410) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_8(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (7566) - 600) / 8 + -rd(d, p + (7569) - 1) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (7569) - 1) / 8 + -rd(d, p + (7566) - 600) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 6698) * 3 / 2 + rd(d, p + (9394) - 15) / 2) as i16 as i32;
    wr(d, p + (9394), -acc);
    acc = (-acc / 2 + rd(d, p + (9394) - 15) + rd(d, p + (9377) - 22) / 2) as i16 as i32;
    wr(d, p + (9377), -acc);
    acc = (-acc / 2 + rd(d, p + (9377) - 22) + rd(d, p + (9353) - 29) / 2) as i16 as i32;
    wr(d, p + (9353), -acc);
    acc = (-acc / 2 + rd(d, p + (9353) - 29) + rd(d, p + (9322) - 45) / 2) as i16 as i32;
    wr(d, p + (9322), -acc);
    acc = (-acc / 2 + rd(d, p + (9322) - 45) + rd(d, p + (9275) - 61) / 2) as i16 as i32;
    wr(d, p + (9275), -acc);
    acc = (-acc / 2 + rd(d, p + (9275) - 61) + rd(d, p + (8348) - 105) / 2) as i16 as i32;
    wr(d, p + (8348), -acc);
    acc = (-acc / 2 + rd(d, p + (8348) - 105) + rd(d, p + (8241) - 149) / 2) as i16 as i32;
    wr(d, p + (8241), -acc);
    acc = (-acc / 2 + rd(d, p + (8241) - 149) + rd(d, p + (8090) - 213) / 2) as i16 as i32;
    wr(d, p + (8090), -acc);
    acc = (-acc / 2 + rd(d, p + (8090) - 213) + rd(d, p + (7875) - 304) / 2) as i16 as i32;
    wr(d, p + (7875), -acc);
    acc = (-acc / 2 + rd(d, p + (7875) - 304)) as i16 as i32;
    wr(d, p + (7569), acc);
    acc = (rd(d, p + (0) - 6698) * 3 / 2 + rd(d, p + (8561) - 17) / 2) as i16 as i32;
    wr(d, p + (8561), -acc);
    acc = (-acc / 2 + rd(d, p + (8561) - 17) + rd(d, p + (8542) - 25) / 2) as i16 as i32;
    wr(d, p + (8542), -acc);
    acc = (-acc / 2 + rd(d, p + (8542) - 25) + rd(d, p + (8515) - 35) / 2) as i16 as i32;
    wr(d, p + (8515), -acc);
    acc = (-acc / 2 + rd(d, p + (8515) - 35) + rd(d, p + (8478) - 52) / 2) as i16 as i32;
    wr(d, p + (8478), -acc);
    acc = (-acc / 2 + rd(d, p + (8478) - 52) + rd(d, p + (8424) - 74) / 2) as i16 as i32;
    wr(d, p + (8424), -acc);
    acc = (-acc / 2 + rd(d, p + (8424) - 74) + rd(d, p + (9212) - 88) / 2) as i16 as i32;
    wr(d, p + (9212), -acc);
    acc = (-acc / 2 + rd(d, p + (9212) - 88) + rd(d, p + (9122) - 123) / 2) as i16 as i32;
    wr(d, p + (9122), -acc);
    acc = (-acc / 2 + rd(d, p + (9122) - 123) + rd(d, p + (8997) - 174) / 2) as i16 as i32;
    wr(d, p + (8997), -acc);
    acc = (-acc / 2 + rd(d, p + (8997) - 174) + rd(d, p + (8821) - 258) / 2) as i16 as i32;
    wr(d, p + (8821), -acc);
    acc = (-acc / 2 + rd(d, p + (8821) - 258)) as i16 as i32;
    wr(d, p + (7566), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_9(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 6328) + rd(d, p + (9704) - 41) / 2) as i16 as i32;
    wr(d, p + (9704), -acc);
    acc = (-acc / 2 + rd(d, p + (9704) - 41) + rd(d, p + (9661) - 195) / 2) as i16 as i32;
    wr(d, p + (9661), -acc);
    acc = (-acc / 2 + rd(d, p + (9661) - 195) + rd(d, p + (9464) - 63) / 2) as i16 as i32;
    wr(d, p + (9464), -acc);
    acc = (-acc / 2 + rd(d, p + (9464) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 6328) + rd(d, p + (10051) - 49) / 2) as i16 as i32;
    wr(d, p + (10051), -acc);
    acc = (-acc / 2 + rd(d, p + (10051) - 49) + rd(d, p + (10000) - 114) / 2) as i16 as i32;
    wr(d, p + (10000), -acc);
    acc = (-acc / 2 + rd(d, p + (10000) - 114) + rd(d, p + (9884) - 178) / 2) as i16 as i32;
    wr(d, p + (9884), -acc);
    acc = (-acc / 2 + rd(d, p + (9884) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (9399) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9399), acc);
    acc = (rd(d, p + (9399) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (9384), acc);
    acc = (rd(d, p + (9393) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (9384) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9393), acc);
    acc = (rd(d, p + (9393) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (9384) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_10(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 5625) + rd(d, p + (10407) - 41) / 2) as i16 as i32;
    wr(d, p + (10407), -acc);
    acc = (-acc / 2 + rd(d, p + (10407) - 41) + rd(d, p + (10364) - 195) / 2) as i16 as i32;
    wr(d, p + (10364), -acc);
    acc = (-acc / 2 + rd(d, p + (10364) - 195) + rd(d, p + (10167) - 63) / 2) as i16 as i32;
    wr(d, p + (10167), -acc);
    acc = (-acc / 2 + rd(d, p + (10167) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 5625) + rd(d, p + (10754) - 49) / 2) as i16 as i32;
    wr(d, p + (10754), -acc);
    acc = (-acc / 2 + rd(d, p + (10754) - 49) + rd(d, p + (10703) - 114) / 2) as i16 as i32;
    wr(d, p + (10703), -acc);
    acc = (-acc / 2 + rd(d, p + (10703) - 114) + rd(d, p + (10587) - 178) / 2) as i16 as i32;
    wr(d, p + (10587), -acc);
    acc = (-acc / 2 + rd(d, p + (10587) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (10102) - 1) / 2) as i16 as i32;
    wr(d, p + (10102), acc);
    acc = (rd(d, p + (10099) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (10102) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (10099), acc);
    acc = (rd(d, p + (10099) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (10102) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_11(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 5156) + rd(d, p + (10876) - 41) / 2) as i16 as i32;
    wr(d, p + (10876), -acc);
    acc = (-acc / 2 + rd(d, p + (10876) - 41) + rd(d, p + (10833) - 195) / 2) as i16 as i32;
    wr(d, p + (10833), -acc);
    acc = (-acc / 2 + rd(d, p + (10833) - 195) + rd(d, p + (10636) - 63) / 2) as i16 as i32;
    wr(d, p + (10636), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (10636) - 63) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 5156) + rd(d, p + (11223) - 49) / 2) as i16 as i32;
    wr(d, p + (11223), -acc);
    acc = (-acc / 2 + rd(d, p + (11223) - 49) + rd(d, p + (11172) - 114) / 2) as i16 as i32;
    wr(d, p + (11172), -acc);
    acc = (-acc / 2 + rd(d, p + (11172) - 114) + rd(d, p + (11056) - 178) / 2) as i16 as i32;
    wr(d, p + (11056), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (11056) - 178) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (10571) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (10571), acc);
    acc = (rd(d, p + (10568) - 1) * 5 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (10571) - 1) * 3 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (10568), acc);
    acc = (rd(d, p + (10568) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_12(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (9855) - 1)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (9852) - 600)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 4687) * 3 / 2 + rd(d, p + (11692) - 15) / 2) as i16 as i32;
    wr(d, p + (11692), -acc);
    acc = (-acc / 2 + rd(d, p + (11692) - 15) + rd(d, p + (11675) - 22) / 2) as i16 as i32;
    wr(d, p + (11675), -acc);
    acc = (-acc / 2 + rd(d, p + (11675) - 22) + rd(d, p + (11651) - 29) / 2) as i16 as i32;
    wr(d, p + (11651), -acc);
    acc = (-acc / 2 + rd(d, p + (11651) - 29) + rd(d, p + (11620) - 45) / 2) as i16 as i32;
    wr(d, p + (11620), -acc);
    acc = (-acc / 2 + rd(d, p + (11620) - 45) + rd(d, p + (11573) - 61) / 2) as i16 as i32;
    wr(d, p + (11573), -acc);
    acc = (-acc / 2 + rd(d, p + (11573) - 61) + rd(d, p + (10600) - 105) / 2) as i16 as i32;
    wr(d, p + (10600), -acc);
    acc = (-acc / 2 + rd(d, p + (10600) - 105) + rd(d, p + (10493) - 149) / 2) as i16 as i32;
    wr(d, p + (10493), -acc);
    acc = (-acc / 2 + rd(d, p + (10493) - 149) + rd(d, p + (10342) - 213) / 2) as i16 as i32;
    wr(d, p + (10342), -acc);
    acc = (-acc / 2 + rd(d, p + (10342) - 213) + rd(d, p + (10127) - 258) / 2) as i16 as i32;
    wr(d, p + (10127), -acc);
    acc = (-acc / 2 + rd(d, p + (10127) - 258)) as i16 as i32;
    wr(d, p + (9855), acc);
    acc = (rd(d, p + (16381) - 4687) * 3 / 2 + rd(d, p + (10813) - 17) / 2) as i16 as i32;
    wr(d, p + (10813), -acc);
    acc = (-acc / 2 + rd(d, p + (10813) - 17) + rd(d, p + (10794) - 25) / 2) as i16 as i32;
    wr(d, p + (10794), -acc);
    acc = (-acc / 2 + rd(d, p + (10794) - 25) + rd(d, p + (10767) - 35) / 2) as i16 as i32;
    wr(d, p + (10767), -acc);
    acc = (-acc / 2 + rd(d, p + (10767) - 35) + rd(d, p + (10730) - 52) / 2) as i16 as i32;
    wr(d, p + (10730), -acc);
    acc = (-acc / 2 + rd(d, p + (10730) - 52) + rd(d, p + (10676) - 74) / 2) as i16 as i32;
    wr(d, p + (10676), -acc);
    acc = (-acc / 2 + rd(d, p + (10676) - 74) + rd(d, p + (11510) - 88) / 2) as i16 as i32;
    wr(d, p + (11510), -acc);
    acc = (-acc / 2 + rd(d, p + (11510) - 88) + rd(d, p + (11420) - 123) / 2) as i16 as i32;
    wr(d, p + (11420), -acc);
    acc = (-acc / 2 + rd(d, p + (11420) - 123) + rd(d, p + (11295) - 174) / 2) as i16 as i32;
    wr(d, p + (11295), -acc);
    acc = (-acc / 2 + rd(d, p + (11295) - 174) + rd(d, p + (11119) - 304) / 2) as i16 as i32;
    wr(d, p + (11119), -acc);
    acc = (-acc / 2 + rd(d, p + (11119) - 304)) as i16 as i32;
    wr(d, p + (9852), acc);
    acc = (rd(d, p + (9867) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9867), acc);
    acc = (rd(d, p + (9861) - 1) * 5 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (9867) - 1) * 3 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9861), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (9861) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    (left, right)
}

fn midifex_effect_13(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_d = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 4224) + rd(d, p + (12158) - 87) / 2) as i16 as i32;
    wr(d, p + (12158), -acc);
    acc = (-acc / 2 + rd(d, p + (12158) - 87) + rd(d, p + (12069) - 103) / 2) as i16 as i32;
    wr(d, p + (12069), -acc);
    acc = (-acc / 2 + rd(d, p + (12069) - 103) + rd(d, p + (11964) - 127) / 2) as i16 as i32;
    wr(d, p + (11964), -acc);
    acc = (-acc / 2 + rd(d, p + (11964) - 127) + rd(d, p + (11835) - 143) / 2) as i16 as i32;
    wr(d, p + (11835), -acc);
    acc = (-acc / 2 + rd(d, p + (11835) - 143) + rd(d, p + (11690) - 174) / 2) as i16 as i32;
    wr(d, p + (11690), -acc);
    acc = (-acc / 2 + rd(d, p + (11690) - 174) + rd(d, p + (11514) - 215) / 2) as i16 as i32;
    wr(d, p + (11514), -acc);
    acc = (-acc / 2 + rd(d, p + (11514) - 215) + rd(d, p + (11297) - 251) / 2) as i16 as i32;
    wr(d, p + (11297), -acc);
    acc = (-acc / 2 + rd(d, p + (11297) - 251) + rd(d, p + (11044) - 300) / 2) as i16 as i32;
    wr(d, p + (11044), -acc);
    acc = (-acc / 2 + rd(d, p + (11044) - 300) + rd(d, p + (10742) - 369) / 2) as i16 as i32;
    wr(d, p + (10742), -acc);
    acc = (-acc / 2 + rd(d, p + (10742) - 369) + rd(d, p + (10371) - 4) / 2) as i16 as i32;
    wr(d, p + (10371), -acc);
    acc = (-acc / 2 + rd(d, p + (10371) - 4) + rd(d, p + (10365) - 52) / 2) as i16 as i32;
    wr(d, p + (10365), -acc);
    acc = (-acc / 2 + rd(d, p + (10365) - 52) + rd(d, p + (10311) - 62) / 2) as i16 as i32;
    wr(d, p + (10311), -acc);
    acc = (-acc / 2 + rd(d, p + (10311) - 62) + rd(d, p + (10247) - 7) / 2) as i16 as i32;
    wr(d, p + (10247), -acc);
    acc = (-acc / 2 + rd(d, p + (10247) - 7) + rd(d, p + (10238) - 13) / 2) as i16 as i32;
    wr(d, p + (10238), -acc);
    acc = (-acc / 2 + rd(d, p + (10238) - 13)) as i16 as i32;
    tmp_d = (acc) as i16 as i32;
    acc = (tmp_d + rd(d, p + (10223) - 36) / 2) as i16 as i32;
    wr(d, p + (10223), -acc);
    acc = (-acc / 2 + rd(d, p + (10223) - 36)) as i16 as i32;
    wr(d, p + (6), -acc);
    acc = (tmp_d + rd(d, p + (10185) - 30) / 2) as i16 as i32;
    wr(d, p + (10185), -acc);
    acc = (-acc / 2 + rd(d, p + (10185) - 30)) as i16 as i32;
    wr(d, p + (4), -acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_14(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 3753)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 3753)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_15(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3515) + rd(d, p + (9174) - 41) / 2) as i16 as i32;
    wr(d, p + (9174), -acc);
    acc = (-acc / 2 + rd(d, p + (9174) - 41) + rd(d, p + (9131) - 195) / 2) as i16 as i32;
    wr(d, p + (9131), -acc);
    acc = (-acc / 2 + rd(d, p + (9131) - 195) + rd(d, p + (8934) - 63) / 2) as i16 as i32;
    wr(d, p + (8934), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (8934) - 63) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 3515) + rd(d, p + (9521) - 49) / 2) as i16 as i32;
    wr(d, p + (9521), -acc);
    acc = (-acc / 2 + rd(d, p + (9521) - 49) + rd(d, p + (9470) - 114) / 2) as i16 as i32;
    wr(d, p + (9470), -acc);
    acc = (-acc / 2 + rd(d, p + (9470) - 114) + rd(d, p + (9354) - 178) / 2) as i16 as i32;
    wr(d, p + (9354), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (9354) - 178) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (8869) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (8869), acc);
    acc = (rd(d, p + (8866) - 1) * 5 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (8869) - 1) * 3 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (8866), acc);
    acc = (rd(d, p + (8866) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_16(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3281) + rd(d, p + (12751) - 41) / 2) as i16 as i32;
    wr(d, p + (12751), -acc);
    acc = (-acc / 2 + rd(d, p + (12751) - 41) + rd(d, p + (12708) - 195) / 2) as i16 as i32;
    wr(d, p + (12708), -acc);
    acc = (-acc / 2 + rd(d, p + (12708) - 195) + rd(d, p + (12511) - 63) / 2) as i16 as i32;
    wr(d, p + (12511), -acc);
    acc = (-acc / 2 + rd(d, p + (12511) - 63)) as i16 as i32;
    wr(d, p + (6), acc);
    acc = (rd(d, p + (16381) - 3281) + rd(d, p + (13098) - 49) / 2) as i16 as i32;
    wr(d, p + (13098), -acc);
    acc = (-acc / 2 + rd(d, p + (13098) - 49) + rd(d, p + (13047) - 114) / 2) as i16 as i32;
    wr(d, p + (13047), -acc);
    acc = (-acc / 2 + rd(d, p + (13047) - 114) + rd(d, p + (12931) - 178) / 2) as i16 as i32;
    wr(d, p + (12931), -acc);
    acc = (-acc / 2 + rd(d, p + (12931) - 178)) as i16 as i32;
    wr(d, p + (4), acc);
    acc = (rd(d, p + (12446) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (12446), acc);
    acc = (rd(d, p + (12446) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (12431), acc);
    acc = (rd(d, p + (12443) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (12431) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (12443), acc);
    acc = (rd(d, p + (12443) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (12431) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_17(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3046) + rd(d, p + (12986) - 41) / 2) as i16 as i32;
    wr(d, p + (12986), -acc);
    acc = (-acc / 2 + rd(d, p + (12986) - 41) + rd(d, p + (12943) - 195) / 2) as i16 as i32;
    wr(d, p + (12943), -acc);
    acc = (-acc / 2 + rd(d, p + (12943) - 195) + rd(d, p + (12746) - 63) / 2) as i16 as i32;
    wr(d, p + (12746), -acc);
    acc = (-acc / 2 + rd(d, p + (12746) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 3046) + rd(d, p + (13333) - 49) / 2) as i16 as i32;
    wr(d, p + (13333), -acc);
    acc = (-acc / 2 + rd(d, p + (13333) - 49) + rd(d, p + (13282) - 114) / 2) as i16 as i32;
    wr(d, p + (13282), -acc);
    acc = (-acc / 2 + rd(d, p + (13282) - 114) + rd(d, p + (13166) - 178) / 2) as i16 as i32;
    wr(d, p + (13166), -acc);
    acc = (-acc / 2 + rd(d, p + (13166) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (12681) - 1) / 2) as i16 as i32;
    wr(d, p + (12681), acc);
    acc = (rd(d, p + (12678) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (12681) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (12678), acc);
    acc = (rd(d, p + (12678) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (12681) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_18(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 2815)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 2815)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_19(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (11684) - 1) / 4 + rd(d, p + (11687) - 1)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (-rd(d, p + (11687) - 1) / 4 + rd(d, p + (11684) - 1)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2580) + rd(d, p + (13512) - 15) / 2) as i16 as i32;
    wr(d, p + (13512), -acc);
    acc = (-acc / 2 + rd(d, p + (13512) - 15) + rd(d, p + (13495) - 22) / 2) as i16 as i32;
    wr(d, p + (13495), -acc);
    acc = (-acc / 2 + rd(d, p + (13495) - 22) + rd(d, p + (13471) - 29) / 2) as i16 as i32;
    wr(d, p + (13471), -acc);
    acc = (-acc / 2 + rd(d, p + (13471) - 29) + rd(d, p + (13440) - 45) / 2) as i16 as i32;
    wr(d, p + (13440), -acc);
    acc = (-acc / 2 + rd(d, p + (13440) - 45) + rd(d, p + (13393) - 61) / 2) as i16 as i32;
    wr(d, p + (13393), -acc);
    acc = (-acc / 2 + rd(d, p + (13393) - 61) + rd(d, p + (12466) - 105) / 2) as i16 as i32;
    wr(d, p + (12466), -acc);
    acc = (-acc / 2 + rd(d, p + (12466) - 105) + rd(d, p + (12359) - 149) / 2) as i16 as i32;
    wr(d, p + (12359), -acc);
    acc = (-acc / 2 + rd(d, p + (12359) - 149) + rd(d, p + (12208) - 213) / 2) as i16 as i32;
    wr(d, p + (12208), -acc);
    acc = (-acc / 2 + rd(d, p + (12208) - 213) + rd(d, p + (11993) - 304) / 2) as i16 as i32;
    wr(d, p + (11993), -acc);
    acc = (-acc / 2 + rd(d, p + (11993) - 304)) as i16 as i32;
    wr(d, p + (11687), acc);
    acc = (rd(d, p + (0) - 2580) + rd(d, p + (12679) - 17) / 2) as i16 as i32;
    wr(d, p + (12679), -acc);
    acc = (-acc / 2 + rd(d, p + (12679) - 17) + rd(d, p + (12660) - 25) / 2) as i16 as i32;
    wr(d, p + (12660), -acc);
    acc = (-acc / 2 + rd(d, p + (12660) - 25) + rd(d, p + (12633) - 35) / 2) as i16 as i32;
    wr(d, p + (12633), -acc);
    acc = (-acc / 2 + rd(d, p + (12633) - 35) + rd(d, p + (12596) - 52) / 2) as i16 as i32;
    wr(d, p + (12596), -acc);
    acc = (-acc / 2 + rd(d, p + (12596) - 52) + rd(d, p + (12542) - 74) / 2) as i16 as i32;
    wr(d, p + (12542), -acc);
    acc = (-acc / 2 + rd(d, p + (12542) - 74) + rd(d, p + (13330) - 88) / 2) as i16 as i32;
    wr(d, p + (13330), -acc);
    acc = (-acc / 2 + rd(d, p + (13330) - 88) + rd(d, p + (13240) - 123) / 2) as i16 as i32;
    wr(d, p + (13240), -acc);
    acc = (-acc / 2 + rd(d, p + (13240) - 123) + rd(d, p + (13115) - 174) / 2) as i16 as i32;
    wr(d, p + (13115), -acc);
    acc = (-acc / 2 + rd(d, p + (13115) - 174) + rd(d, p + (12939) - 258) / 2) as i16 as i32;
    wr(d, p + (12939), -acc);
    acc = (-acc / 2 + rd(d, p + (12939) - 258)) as i16 as i32;
    wr(d, p + (11684), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_20(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 2346)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 2346)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_21(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (13597) - 1) / 4 + rd(d, p + (13600) - 1)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (-rd(d, p + (13600) - 1) / 4 + rd(d, p + (13597) - 1)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 2109) * 3 / 2 + rd(d, p + (14229) - 29) / 2) as i16 as i32;
    wr(d, p + (14229), -acc);
    acc = (-acc / 2 + rd(d, p + (14229) - 29) + rd(d, p + (14198) - 45) / 2) as i16 as i32;
    wr(d, p + (14198), -acc);
    acc = (-acc / 2 + rd(d, p + (14198) - 45) + rd(d, p + (14151) - 61) / 2) as i16 as i32;
    wr(d, p + (14151), -acc);
    acc = (-acc / 2 + rd(d, p + (14151) - 61) + rd(d, p + (13756) - 99) / 2) as i16 as i32;
    wr(d, p + (13756), -acc);
    acc = (-acc / 2 + rd(d, p + (13756) - 99) + rd(d, p + (13655) - 14) / 2) as i16 as i32;
    wr(d, p + (13655), -acc);
    acc = (-acc / 2 + rd(d, p + (13655) - 14) + rd(d, p + (13639) - 21) / 2) as i16 as i32;
    wr(d, p + (13639), -acc);
    acc = (-acc / 2 + rd(d, p + (13639) - 21) + rd(d, p + (13616) - 2) / 2) as i16 as i32;
    wr(d, p + (13616), -acc);
    acc = (-acc / 2 + rd(d, p + (13616) - 2)) as i16 as i32;
    wr(d, p + (13600), acc);
    acc = (rd(d, p + (16381) - 2109) * 3 / 2 + rd(d, p + (13923) - 35) / 2) as i16 as i32;
    wr(d, p + (13923), -acc);
    acc = (-acc / 2 + rd(d, p + (13923) - 35) + rd(d, p + (13886) - 52) / 2) as i16 as i32;
    wr(d, p + (13886), -acc);
    acc = (-acc / 2 + rd(d, p + (13886) - 52) + rd(d, p + (13832) - 74) / 2) as i16 as i32;
    wr(d, p + (13832), -acc);
    acc = (-acc / 2 + rd(d, p + (13832) - 74) + rd(d, p + (14088) - 88) / 2) as i16 as i32;
    wr(d, p + (14088), -acc);
    acc = (-acc / 2 + rd(d, p + (14088) - 88) + rd(d, p + (13998) - 12) / 2) as i16 as i32;
    wr(d, p + (13998), -acc);
    acc = (-acc / 2 + rd(d, p + (13998) - 12) + rd(d, p + (13984) - 8) / 2) as i16 as i32;
    wr(d, p + (13984), -acc);
    acc = (-acc / 2 + rd(d, p + (13984) - 8) + rd(d, p + (13974) - 3) / 2) as i16 as i32;
    wr(d, p + (13974), -acc);
    acc = (-acc / 2 + rd(d, p + (13974) - 3)) as i16 as i32;
    wr(d, p + (13597), acc);
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (13612) - 1) / 2) as i16 as i32;
    wr(d, p + (13612), acc);
    acc = (rd(d, p + (13609) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (13612) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (13609), acc);
    acc = (rd(d, p + (13609) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (13612) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16381), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_22(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 15705) / 2 + rd(d, p + (0) - 7853) + rd(d, p + (330) - 41) / 2) as i16 as i32;
    wr(d, p + (330), -acc);
    acc = (-acc / 2 + rd(d, p + (330) - 41) + rd(d, p + (287) - 195) / 2) as i16 as i32;
    wr(d, p + (287), -acc);
    acc = (-acc / 2 + rd(d, p + (287) - 195) + rd(d, p + (90) - 63) / 2) as i16 as i32;
    wr(d, p + (90), -acc);
    acc = (-acc / 2 + rd(d, p + (90) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 15705) / 2 + rd(d, p + (0) - 7853) + rd(d, p + (677) - 49) / 2) as i16 as i32;
    wr(d, p + (677), -acc);
    acc = (-acc / 2 + rd(d, p + (677) - 49) + rd(d, p + (626) - 114) / 2) as i16 as i32;
    wr(d, p + (626), -acc);
    acc = (-acc / 2 + rd(d, p + (626) - 114) + rd(d, p + (510) - 178) / 2) as i16 as i32;
    wr(d, p + (510), -acc);
    acc = (-acc / 2 + rd(d, p + (510) - 178)) as i16 as i32;
    wr(d, p + (3), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_23(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 6093) / 2 + rd(d, p + (10286) - 6093) / 2 + rd(d, p + (3844) - 41) / 2) as i16 as i32;
    wr(d, p + (3844), -acc);
    acc = (-acc / 2 + rd(d, p + (3844) - 41) + rd(d, p + (3801) - 195) / 2) as i16 as i32;
    wr(d, p + (3801), -acc);
    acc = (-acc / 2 + rd(d, p + (3801) - 195) + rd(d, p + (3604) - 63) / 2) as i16 as i32;
    wr(d, p + (3604), -acc);
    acc = (-acc / 2 + rd(d, p + (3604) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 6093) / 2 + rd(d, p + (10286) - 6093) / 2 + rd(d, p + (4191) - 49) / 2) as i16 as i32;
    wr(d, p + (4191), -acc);
    acc = (-acc / 2 + rd(d, p + (4191) - 49) + rd(d, p + (4140) - 114) / 2) as i16 as i32;
    wr(d, p + (4140), -acc);
    acc = (-acc / 2 + rd(d, p + (4140) - 114) + rd(d, p + (4024) - 178) / 2) as i16 as i32;
    wr(d, p + (4024), -acc);
    acc = (-acc / 2 + rd(d, p + (4024) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (3539) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (3539), acc);
    acc = (rd(d, p + (3539) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (3536) - 1) * 13 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 6093) * 3 / 16 + tmp_1) as i16 as i32;
    wr(d, p + (3536), acc);
    acc = (rd(d, p + (3536) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 6093) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (10286), acc);
    acc = (rd(d, p + (3533) - 1) / 4) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3533) - 7) * 3 / 4 + tmp_1) as i16 as i32;
    wr(d, p + (3533), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_24(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 5390) / 2 + rd(d, p + (10989) - 5390) / 2 + rd(d, p + (5250) - 41) / 2) as i16 as i32;
    wr(d, p + (5250), -acc);
    acc = (-acc / 2 + rd(d, p + (5250) - 41) + rd(d, p + (5207) - 195) / 2) as i16 as i32;
    wr(d, p + (5207), -acc);
    acc = (-acc / 2 + rd(d, p + (5207) - 195) + rd(d, p + (5010) - 63) / 2) as i16 as i32;
    wr(d, p + (5010), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5010) - 63) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 5390) / 2 + rd(d, p + (10989) - 5390) / 2 + rd(d, p + (5597) - 49) / 2) as i16 as i32;
    wr(d, p + (5597), -acc);
    acc = (-acc / 2 + rd(d, p + (5597) - 49) + rd(d, p + (5546) - 114) / 2) as i16 as i32;
    wr(d, p + (5546), -acc);
    acc = (-acc / 2 + rd(d, p + (5546) - 114) + rd(d, p + (5430) - 178) / 2) as i16 as i32;
    wr(d, p + (5430), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (5430) - 178) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (4945) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (4945), acc);
    acc = (rd(d, p + (4942) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (4945) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (4942), acc);
    acc = (rd(d, p + (4942) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (4945) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (16381) - 5390) / 2 + rd(d, p + (4939) - 1) / 2) as i16 as i32;
    wr(d, p + (4939), acc);
    acc = (rd(d, p + (4936) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (4939) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (4936), acc);
    acc = (rd(d, p + (4936) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (4939) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (10989), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_25(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_d = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (0) - 8919) / 4 + rd(d, p + (0) - 4210) / 2 + rd(d, p + (7463) - 87) / 2) as i16 as i32;
    wr(d, p + (7463), -acc);
    acc = (-acc / 2 + rd(d, p + (7463) - 87) + rd(d, p + (7374) - 103) / 2) as i16 as i32;
    wr(d, p + (7374), -acc);
    acc = (-acc / 2 + rd(d, p + (7374) - 103) + rd(d, p + (7269) - 127) / 2) as i16 as i32;
    wr(d, p + (7269), -acc);
    acc = (-acc / 2 + rd(d, p + (7269) - 127) + rd(d, p + (7140) - 143) / 2) as i16 as i32;
    wr(d, p + (7140), -acc);
    acc = (-acc / 2 + rd(d, p + (7140) - 143) + rd(d, p + (6995) - 174) / 2) as i16 as i32;
    wr(d, p + (6995), -acc);
    acc = (-acc / 2 + rd(d, p + (6995) - 174) + rd(d, p + (6819) - 215) / 2) as i16 as i32;
    wr(d, p + (6819), -acc);
    acc = (-acc / 2 + rd(d, p + (6819) - 215) + rd(d, p + (6602) - 251) / 2) as i16 as i32;
    wr(d, p + (6602), -acc);
    acc = (-acc / 2 + rd(d, p + (6602) - 251) + rd(d, p + (6349) - 300) / 2) as i16 as i32;
    wr(d, p + (6349), -acc);
    acc = (-acc / 2 + rd(d, p + (6349) - 300) + rd(d, p + (6047) - 369) / 2) as i16 as i32;
    wr(d, p + (6047), -acc);
    acc = (-acc / 2 + rd(d, p + (6047) - 369)) as i16 as i32;
    tmp_d = (acc) as i16 as i32;
    acc = (tmp_d + rd(d, p + (5469) - 175) / 2) as i16 as i32;
    wr(d, p + (5469), -acc);
    acc = (-acc / 2 + rd(d, p + (5469) - 175) + rd(d, p + (5133) - 36) / 2) as i16 as i32;
    wr(d, p + (5133), -acc);
    acc = (-acc / 2 + rd(d, p + (5133) - 36)) as i16 as i32;
    wr(d, p + (6), -acc);
    acc = (tmp_d + rd(d, p + (5292) - 157) / 2) as i16 as i32;
    wr(d, p + (5292), -acc);
    acc = (-acc / 2 + rd(d, p + (5292) - 157) + rd(d, p + (5095) - 30) / 2) as i16 as i32;
    wr(d, p + (5095), -acc);
    acc = (-acc / 2 + rd(d, p + (5095) - 30)) as i16 as i32;
    wr(d, p + (4), -acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_26(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 4101) + rd(d, p + (0) - 8204) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 3890) + rd(d, p + (0) - 8626) / 2) as i16 as i32;
    wr(d, p + (3), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_27(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3515) + rd(d, p + (9000) - 41) / 2) as i16 as i32;
    wr(d, p + (9000), -acc);
    acc = (-acc / 2 + rd(d, p + (9000) - 41) + rd(d, p + (8957) - 195) / 2) as i16 as i32;
    wr(d, p + (8957), -acc);
    acc = (-acc / 2 + rd(d, p + (8957) - 195) + rd(d, p + (8760) - 63) / 2) as i16 as i32;
    wr(d, p + (8760), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (8760) - 63) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (12864) - 3515) + rd(d, p + (9347) - 49) / 2) as i16 as i32;
    wr(d, p + (9347), -acc);
    acc = (-acc / 2 + rd(d, p + (9347) - 49) + rd(d, p + (9296) - 114) / 2) as i16 as i32;
    wr(d, p + (9296), -acc);
    acc = (-acc / 2 + rd(d, p + (9296) - 114) + rd(d, p + (9180) - 178) / 2) as i16 as i32;
    wr(d, p + (9180), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (9180) - 178) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (8695) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (8695), acc);
    acc = (rd(d, p + (8695) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (8692) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 3515) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (8692), acc);
    acc = (rd(d, p + (8692) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 3515) + tmp_1) as i16 as i32;
    wr(d, p + (12864), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_28(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3046) + rd(d, p + (13333) - 3046) / 2 + rd(d, p + (9938) - 13) / 2) as i16 as i32;
    wr(d, p + (9938), -acc);
    acc = (-acc / 2 + rd(d, p + (9938) - 13) + rd(d, p + (9923) - 195) / 2) as i16 as i32;
    wr(d, p + (9923), -acc);
    acc = (-acc / 2 + rd(d, p + (9923) - 195) + rd(d, p + (9726) - 63) / 2) as i16 as i32;
    wr(d, p + (9726), -acc);
    acc = (-acc / 2 + rd(d, p + (9726) - 63)) as i16 as i32;
    wr(d, p + (401), acc);
    acc = (rd(d, p + (16381) - 3046) + rd(d, p + (13333) - 3046) / 2 + rd(d, p + (10285) - 49) / 2) as i16 as i32;
    wr(d, p + (10285), -acc);
    acc = (-acc / 2 + rd(d, p + (10285) - 49) + rd(d, p + (10234) - 114) / 2) as i16 as i32;
    wr(d, p + (10234), -acc);
    acc = (-acc / 2 + rd(d, p + (10234) - 114) + rd(d, p + (10118) - 178) / 2) as i16 as i32;
    wr(d, p + (10118), -acc);
    acc = (-acc / 2 + rd(d, p + (10118) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (9661) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9661), acc);
    acc = (rd(d, p + (9658) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (9661) - 1) / 16 + tmp_1) as i16 as i32;
    wr(d, p + (9658), acc);
    acc = (rd(d, p + (9658) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (9661) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (16381) - 3046) / 2 + rd(d, p + (9655) - 1) / 2) as i16 as i32;
    wr(d, p + (9655), acc);
    acc = (rd(d, p + (9652) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (9655) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9652), acc);
    acc = (rd(d, p + (9652) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (9655) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (13333), acc);
    left = (rd(d, p + (401) - 396)) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_29(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 2695) + rd(d, p + (13684) - 2695) + rd(d, p + (10640) - 41) / 2) as i16 as i32;
    wr(d, p + (10640), -acc);
    acc = (-acc / 2 + rd(d, p + (10640) - 41) + rd(d, p + (10597) - 195) / 2) as i16 as i32;
    wr(d, p + (10597), -acc);
    acc = (-acc / 2 + rd(d, p + (10597) - 195) + rd(d, p + (10400) - 63) / 2) as i16 as i32;
    wr(d, p + (10400), -acc);
    acc = (-acc / 2 + rd(d, p + (10400) - 63)) as i16 as i32;
    wr(d, p + (6), acc);
    acc = (rd(d, p + (16381) - 2695) + rd(d, p + (13684) - 2695) + rd(d, p + (10987) - 49) / 2) as i16 as i32;
    wr(d, p + (10987), -acc);
    acc = (-acc / 2 + rd(d, p + (10987) - 49) + rd(d, p + (10936) - 114) / 2) as i16 as i32;
    wr(d, p + (10936), -acc);
    acc = (-acc / 2 + rd(d, p + (10936) - 114) + rd(d, p + (10820) - 178) / 2) as i16 as i32;
    wr(d, p + (10820), -acc);
    acc = (-acc / 2 + rd(d, p + (10820) - 178)) as i16 as i32;
    wr(d, p + (4), acc);
    acc = (rd(d, p + (10335) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (10335), acc);
    acc = (rd(d, p + (10335) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (10332) - 1) / 2 + rd(d, p + (16381) - 2695) / 2) as i16 as i32;
    wr(d, p + (10332), acc);
    acc = (rd(d, p + (10332) - 1)) as i16 as i32;
    wr(d, p + (13684), acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_30(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 2343) * 3 / 4 + rd(d, p + (0) - 4688) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2343) * 3 / 4 + rd(d, p + (0) - 4688) * 3 / 4) as i16 as i32;
    wr(d, p + (551), acc);
    left = (rd(d, p + (551) - 546)) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_31(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5460) + rd(d, p + (0) - 16378) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 10919) + rd(d, p + (0) - 16378) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_32(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 4247) + rd(d, p + (0) - 12739) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 8493) + rd(d, p + (0) - 12739) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_33(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 4244) / 2 + rd(d, p + (12135) - 4244) / 2 + rd(d, p + (7889) - 4244) / 2 + rd(d, p + (3296) - 41) / 2) as i16 as i32;
    wr(d, p + (3296), -acc);
    acc = (-acc / 2 + rd(d, p + (3296) - 41) + rd(d, p + (3253) - 195) / 2) as i16 as i32;
    wr(d, p + (3253), -acc);
    acc = (-acc / 2 + rd(d, p + (3253) - 195) + rd(d, p + (3056) - 63) / 2) as i16 as i32;
    wr(d, p + (3056), -acc);
    acc = (-acc / 2 + rd(d, p + (3056) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (7889) - 4244) / 2 + rd(d, p + (12135) - 4244) / 2 + rd(d, p + (16381) - 4244) / 2 + rd(d, p + (3643) - 49) / 2) as i16 as i32;
    wr(d, p + (3643), -acc);
    acc = (-acc / 2 + rd(d, p + (3643) - 49) + rd(d, p + (3592) - 114) / 2) as i16 as i32;
    wr(d, p + (3592), -acc);
    acc = (-acc / 2 + rd(d, p + (3592) - 114) + rd(d, p + (3476) - 178) / 2) as i16 as i32;
    wr(d, p + (3476), -acc);
    acc = (-acc / 2 + rd(d, p + (3476) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (2985) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (2985), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    wr(d, p + (7889), acc);
    acc = (rd(d, p + (2988) - 1) / 2 + rd(d, p + (7889) - 4244) / 2) as i16 as i32;
    wr(d, p + (2988), acc);
    acc = (rd(d, p + (2988) - 1)) as i16 as i32;
    wr(d, p + (12135), acc);
    acc = (rd(d, p + (2991) - 1) / 2 + rd(d, p + (12135) - 4244) / 2) as i16 as i32;
    wr(d, p + (2991), acc);
    acc = (rd(d, p + (2991) - 0)) as i16 as i32;
    wr(d, p + (16381), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_34(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16381) - 3276) / 2 + rd(d, p + (13103) - 3276) / 2 + rd(d, p + (9825) - 3276) / 2 + rd(d, p + (6200) - 41) / 2) as i16 as i32;
    wr(d, p + (6200), -acc);
    acc = (-acc / 2 + rd(d, p + (6200) - 41) + rd(d, p + (6157) - 195) / 2) as i16 as i32;
    wr(d, p + (6157), -acc);
    acc = (-acc / 2 + rd(d, p + (6157) - 195) + rd(d, p + (5960) - 63) / 2) as i16 as i32;
    wr(d, p + (5960), -acc);
    acc = (-acc / 2 + rd(d, p + (5960) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 3276) / 2 + rd(d, p + (13103) - 3276) / 2 + rd(d, p + (9825) - 3276) / 2 + rd(d, p + (6547) - 49) / 2) as i16 as i32;
    wr(d, p + (6547), -acc);
    acc = (-acc / 2 + rd(d, p + (6547) - 49) + rd(d, p + (6496) - 114) / 2) as i16 as i32;
    wr(d, p + (6496), -acc);
    acc = (-acc / 2 + rd(d, p + (6496) - 114) + rd(d, p + (6380) - 178) / 2) as i16 as i32;
    wr(d, p + (6380), -acc);
    acc = (-acc / 2 + rd(d, p + (6380) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (5895) - 1) * 3 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 5 / 8 + tmp_1) as i16 as i32;
    wr(d, p + (5895), acc);
    acc = (rd(d, p + (5892) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (5895) - 1) / 16 + tmp_1) as i16 as i32;
    wr(d, p + (5892), acc);
    acc = (rd(d, p + (5892) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (5895) - 1) + tmp_1) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (5889) - 1) / 2 + rd(d, p + (16381) - 3276) / 2) as i16 as i32;
    wr(d, p + (5889), acc);
    acc = (rd(d, p + (5886) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (5889) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (5886), acc);
    acc = (rd(d, p + (5886) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (5889) - 1) * 9 / 4 + tmp_1 * 9 / 4) as i16 as i32;
    wr(d, p + (13103), acc);
    acc = (rd(d, p + (5883) - 1) / 2 + rd(d, p + (13103) - 3276) / 2) as i16 as i32;
    wr(d, p + (5883), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (5880) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (5883) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (5880), acc);
    acc = (rd(d, p + (5880) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (5883) - 1) * 9 / 4 + tmp_1 * 9 / 4) as i16 as i32;
    wr(d, p + (9825), acc);
    (left, right)
}

fn midifex_effect_35(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (10761) - 2808) / 2 + rd(d, p + (13571) - 2808) / 2 + rd(d, p + (16381) - 2808) / 2 + rd(d, p + (7604) - 41) / 2) as i16 as i32;
    wr(d, p + (7604), -acc);
    acc = (-acc / 2 + rd(d, p + (7604) - 41) + rd(d, p + (7561) - 195) / 2) as i16 as i32;
    wr(d, p + (7561), -acc);
    acc = (-acc / 2 + rd(d, p + (7561) - 195) + rd(d, p + (7364) - 63) / 2) as i16 as i32;
    wr(d, p + (7364), -acc);
    acc = (-acc / 2 + rd(d, p + (7364) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 2808) / 2 + rd(d, p + (13571) - 2808) / 2 + rd(d, p + (10761) - 2808) / 2 + rd(d, p + (7951) - 49) / 2) as i16 as i32;
    wr(d, p + (7951), -acc);
    acc = (-acc / 2 + rd(d, p + (7951) - 49) + rd(d, p + (7900) - 114) / 2) as i16 as i32;
    wr(d, p + (7900), -acc);
    acc = (-acc / 2 + rd(d, p + (7900) - 114) + rd(d, p + (7784) - 178) / 2) as i16 as i32;
    wr(d, p + (7784), -acc);
    acc = (-acc / 2 + rd(d, p + (7784) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (7299) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 16 + tmp_1) as i16 as i32;
    wr(d, p + (7299), acc);
    acc = (rd(d, p + (7299) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16381), acc);
    acc = (rd(d, p + (7296) - 1) * 15 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (acc * -3 / 16 + rd(d, p + (16381) - 2808) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (7296), acc);
    acc = (rd(d, p + (7296) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 2808) + tmp_1) as i16 as i32;
    wr(d, p + (13571), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (7293) - 1) * 13 / 16) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (13571) - 2808) * 3 / 16 + tmp_1) as i16 as i32;
    wr(d, p + (7293), acc);
    acc = (rd(d, p + (7293) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (13571) - 2808) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (10761), acc);
    (left, right)
}

fn midifex_effect_36(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7027) / 2 + rd(d, p + (0) - 4685) / 2 + rd(d, p + (0) - 2343) / 2 + rd(d, p + (9008) - 41) / 2) as i16 as i32;
    wr(d, p + (9008), -acc);
    acc = (-acc / 2 + rd(d, p + (9008) - 41) + rd(d, p + (8965) - 195) / 2) as i16 as i32;
    wr(d, p + (8965), -acc);
    acc = (-acc / 2 + rd(d, p + (8965) - 195) + rd(d, p + (8768) - 63) / 2) as i16 as i32;
    wr(d, p + (8768), -acc);
    acc = (-acc / 2 + rd(d, p + (8768) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2343) / 2 + rd(d, p + (0) - 4685) / 2 + rd(d, p + (0) - 7027) / 2 + rd(d, p + (9355) - 49) / 2) as i16 as i32;
    wr(d, p + (9355), -acc);
    acc = (-acc / 2 + rd(d, p + (9355) - 49) + rd(d, p + (9304) - 114) / 2) as i16 as i32;
    wr(d, p + (9304), -acc);
    acc = (-acc / 2 + rd(d, p + (9304) - 114) + rd(d, p + (9188) - 178) / 2) as i16 as i32;
    wr(d, p + (9188), -acc);
    acc = (-acc / 2 + rd(d, p + (9188) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_37(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 14000)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (16380) - 14000)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) + rd(d, p + (16380) - 14000) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_38(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_3ffe = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 8171) / 2 + rd(d, p + (8201) - 49) / 2) as i16 as i32;
    wr(d, p + (8201), -acc);
    acc = (-acc / 2 + rd(d, p + (8201) - 49) + rd(d, p + (8150) - 114) / 2) as i16 as i32;
    wr(d, p + (8150), -acc);
    acc = (-acc / 2 + rd(d, p + (8150) - 114) + rd(d, p + (8034) - 178) / 2) as i16 as i32;
    wr(d, p + (8034), -acc);
    acc = (-acc / 2 + rd(d, p + (8034) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 8171) / 2 + rd(d, p + (7854) - 41) / 2) as i16 as i32;
    wr(d, p + (7854), -acc);
    acc = (-acc / 2 + rd(d, p + (7854) - 41) + rd(d, p + (7811) - 195) / 2) as i16 as i32;
    wr(d, p + (7811), -acc);
    acc = (-acc / 2 + rd(d, p + (7811) - 195) + rd(d, p + (7614) - 63) / 2) as i16 as i32;
    wr(d, p + (7614), -acc);
    acc = (-acc / 2 + rd(d, p + (7614) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 8171) / 2 + rd(d, p + (0) - 2) * 3 / 2) as i16 as i32;
    tmp_3ffe = (acc) as i16 as i32;
    acc = (tmp_3ffe / 2) as i16 as i32;
    wr(d, p + (8207), acc);
    acc = (rd(d, p + (8207) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (tmp_3ffe + tmp_1) as i16 as i32;
    wr(d, p + (16380), acc);
    acc = (rd(d, p + (16380) - 0) * 3 / 4) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_39(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_3ffe = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 7020) / 2 + rd(d, p + (9352) - 49) / 2) as i16 as i32;
    wr(d, p + (9352), -acc);
    acc = (-acc / 2 + rd(d, p + (9352) - 49) + rd(d, p + (9301) - 114) / 2) as i16 as i32;
    wr(d, p + (9301), -acc);
    acc = (-acc / 2 + rd(d, p + (9301) - 114) + rd(d, p + (9185) - 178) / 2) as i16 as i32;
    wr(d, p + (9185), -acc);
    acc = (-acc / 2 + rd(d, p + (9185) - 178)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 7020) / 2 + rd(d, p + (9005) - 41) / 2) as i16 as i32;
    wr(d, p + (9005), -acc);
    acc = (-acc / 2 + rd(d, p + (9005) - 41) + rd(d, p + (8962) - 195) / 2) as i16 as i32;
    wr(d, p + (8962), -acc);
    acc = (-acc / 2 + rd(d, p + (8962) - 195) + rd(d, p + (8765) - 63) / 2) as i16 as i32;
    wr(d, p + (8765), -acc);
    acc = (-acc / 2 + rd(d, p + (8765) - 63)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (-rd(d, p + (0) - 2) + -rd(d, p + (16380) - 7020) / 4) as i16 as i32;
    tmp_3ffe = (acc) as i16 as i32;
    acc = (tmp_3ffe / 2 + rd(d, p + (9358) - 1) / 2) as i16 as i32;
    wr(d, p + (9358), acc);
    acc = (rd(d, p + (9355) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (9358) - 1) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (9355), acc);
    acc = (rd(d, p + (9355) - 1)) as i16 as i32;
    tmp_1 = (-acc) as i16 as i32;
    acc = (rd(d, p + (9358) - 1) * 3 / 2 + tmp_1 * 3 / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    acc = (rd(d, p + (8688) - 6) / 2 + rd(d, p + (8691) - 1) / 2) as i16 as i32;
    wr(d, p + (8691), acc);
    acc = (rd(d, p + (8688) - 1) * 7 / 8) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (8688) - 7) / 8 + tmp_1) as i16 as i32;
    wr(d, p + (8688), acc);
    acc = (rd(d, p + (16380) - 0) * 11 / 8) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_40(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16382) - 5002) / 2 + rd(d, p + (11375) - 59) / 2) as i16 as i32;
    wr(d, p + (11375), -acc);
    acc = (-acc / 2 + rd(d, p + (11375) - 59) + rd(d, p + (11314) - 117) / 2) as i16 as i32;
    wr(d, p + (11314), -acc);
    acc = (-acc / 2 + rd(d, p + (11314) - 117) + rd(d, p + (11195) - 139) / 2) as i16 as i32;
    wr(d, p + (11195), -acc);
    acc = (-acc / 2 + rd(d, p + (11195) - 139)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16382) - 5002) / 2 + rd(d, p + (11054) - 45) / 2) as i16 as i32;
    wr(d, p + (11054), -acc);
    acc = (-acc / 2 + rd(d, p + (11054) - 45) + rd(d, p + (11007) - 187) / 2) as i16 as i32;
    wr(d, p + (11007), -acc);
    acc = (-acc / 2 + rd(d, p + (11007) - 187) + rd(d, p + (10818) - 121) / 2) as i16 as i32;
    wr(d, p + (10818), -acc);
    acc = (-acc / 2 + rd(d, p + (10818) - 121)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) * 9 / 8 + rd(d, p + (11378) - 1) * 3 / 4) as i16 as i32;
    wr(d, p + (16382), acc);
    acc = (rd(d, p + (11378) - 1) / 2 + rd(d, p + (16382) - 5002) / 2) as i16 as i32;
    wr(d, p + (11378), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_41(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 4500) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 4500) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (-rd(d, p + (16380) - 4500) / 4 + rd(d, p + (0) - 2)) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_42(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 3000)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (16380) - 3000)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) + rd(d, p + (16380) - 3000) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_43(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 588)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 588)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_44(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 940)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 940)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_45(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1409)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 1409)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_46(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1760)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 1760)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_47(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 2346)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 2346)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_48(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 153) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 153) + rd(d, p + (2120) - 273) / 2) as i16 as i32;
    wr(d, p + (2120), -acc);
    acc = (-acc / 2 + rd(d, p + (2120) - 273)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1845) - 32) / 2) as i16 as i32;
    wr(d, p + (1845), -acc);
    acc = (-acc / 2 + rd(d, p + (1845) - 32) + rd(d, p + (1811) - 181) / 2) as i16 as i32;
    wr(d, p + (1811), -acc);
    acc = (-acc / 2 + rd(d, p + (1811) - 181) + rd(d, p + (1628) - 233) / 2) as i16 as i32;
    wr(d, p + (1628), -acc);
    acc = (-acc / 2 + rd(d, p + (1628) - 233)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 2 + rd(d, p + (0) - 233) / 2 + rd(d, p + (0) - 412) / 2 + rd(d, p + (0) - 534) / 2 + rd(d, p + (0) - 761) / 2 + rd(d, p + (0) - 875) / 2 + rd(d, p + (15383) - 64) / 2 + rd(d, p + (15383) - 118) / 2 + rd(d, p + (15383) - 232) / 2 + rd(d, p + (15383) - 433) / 2 + rd(d, p + (15383) - 554) / 2 + rd(d, p + (15383) - 741) / 2 + rd(d, p + (15383) - 861) / 2 + rd(d, p + (15383) - 1045) / 2 + rd(d, p + (15383) - 1148) / 2 + rd(d, p + (15383) - 1255) / 2 + rd(d, p + (15383) - 1442) / 2 + rd(d, p + (15383) - 1578) / 2 + rd(d, p + (15383) - 1720) / 2 + rd(d, p + (15383) - 1872) / 2 + rd(d, p + (15383) - 2035) / 2 + rd(d, p + (15383) - 2120) / 2 + rd(d, p + (15383) - 2266) / 2 + rd(d, p + (15383) - 2450) / 2 + rd(d, p + (15383) - 2568) / 2 + rd(d, p + (15383) - 2724) / 2 + rd(d, p + (15383) - 2888) / 2 + rd(d, p + (15383) - 3033) / 2 + rd(d, p + (15383) - 3151) / 2 + rd(d, p + (15383) - 3256) / 2 + rd(d, p + (15383) - 3491) / 2 + rd(d, p + (15383) - 3555) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    left = (tmp_5) as i16;
    acc = (-rd(d, p + (15383) - 8094) / 16 + -rd(d, p + (15383) - 7848) / 16 + -rd(d, p + (15383) - 7792) / 16 + -rd(d, p + (15383) - 7514) / 16 + -rd(d, p + (15383) - 7461) / 16 + -rd(d, p + (15383) - 7378) / 16 + -rd(d, p + (15383) - 7138) / 16 + -rd(d, p + (15383) - 7091) / 16 + rd(d, p + (15383) - 6864) / 8 + rd(d, p + (15383) - 6726) / 8 + rd(d, p + (15383) - 6656) / 8 + rd(d, p + (15383) - 6418) / 8 + rd(d, p + (15383) - 6363) / 8 + rd(d, p + (15383) - 6153) / 8 + rd(d, p + (15383) - 6017) / 8 + rd(d, p + (15383) - 5966) / 8 + -rd(d, p + (15383) - 5727) / 4 + -rd(d, p + (15383) - 5656) / 4 + -rd(d, p + (15383) - 5494) / 4 + -rd(d, p + (15383) - 5318) / 4 + -rd(d, p + (15383) - 5260) / 4 + -rd(d, p + (15383) - 5018) / 4 + -rd(d, p + (15383) - 4957) / 4 + -rd(d, p + (15383) - 4733) / 4 + rd(d, p + (15383) - 4626) / 2 + rd(d, p + (15383) - 4550) / 2 + rd(d, p + (15383) - 4338) / 2 + rd(d, p + (15383) - 4209) / 2 + rd(d, p + (15383) - 4045) / 2 + rd(d, p + (15383) - 3920) / 2 + rd(d, p + (15383) - 3799) / 2 + rd(d, p + (15383) - 3618) / 2 + rd(d, p + (1393) - 1) / 2) as i16 as i32;
    wr(d, p + (1393), acc);
    acc = (-rd(d, p + (1393) - 1) / 4 + rd(d, p + (11) - 0)) as i16 as i32;
    wr(d, p + (11), acc);
    acc = (rd(d, p + (0) - 1000) / 2 + rd(d, p + (1390) - 1) / 2) as i16 as i32;
    wr(d, p + (1390), acc);
    acc = (rd(d, p + (1390) - 1)) as i16 as i32;
    wr(d, p + (15383), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_49(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15253) - 64) / 2 + rd(d, p + (13283) - 696) / 2 + rd(d, p + (8586) - 539) / 2 + rd(d, p + (16230) - 24) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15253) - 503) / 2 + rd(d, p + (13283) - 167) / 2 + rd(d, p + (8586) - 151) / 2 + rd(d, p + (16045) - 172) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) * 3 / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (rd(d, p + (16045) - 321) * 5 / 8 + rd(d, p + (16230) - 0) * 5 / 8 + rd(d, p + (16230) - 183)) as i16 as i32;
    wr(d, p + (16045), -acc);
    acc = (rd(d, p + (15722) - 467) * 5 / 8 + rd(d, p + (16045) - 0) * 5 / 8 + rd(d, p + (16045) - 321)) as i16 as i32;
    wr(d, p + (15722), -acc);
    acc = (-acc / 8 + rd(d, p + (15722) - 0) / 2 + rd(d, p + (15722) - 467)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15253) - 845) * 3 / 4 + rd(d, p + (14406) - 1121) / 2) as i16 as i32;
    wr(d, p + (14406), -acc);
    acc = (-acc / 2 + rd(d, p + (14406) - 1121)) as i16 as i32;
    wr(d, p + (13283), acc);
    acc = (rd(d, p + (13283) - 1441) * 3 / 4 + rd(d, p + (11840) - 1320) / 2) as i16 as i32;
    wr(d, p + (11840), -acc);
    acc = (-acc / 2 + rd(d, p + (11840) - 1320)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6264) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6264), acc);
    acc = (rd(d, p + (6264) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15253), acc);
    acc = (rd(d, p + (10518) - 912) * 3 / 4 + rd(d, p + (9604) - 1016) / 2) as i16 as i32;
    wr(d, p + (9604), -acc);
    acc = (-acc / 2 + rd(d, p + (9604) - 1016)) as i16 as i32;
    wr(d, p + (8586), acc);
    acc = (rd(d, p + (8586) - 1017) * 3 / 4 + rd(d, p + (7567) - 1298) / 2) as i16 as i32;
    wr(d, p + (7567), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (7567) - 1298)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6261) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6261), acc);
    acc = (rd(d, p + (6261) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (10518), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_50(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (2190) - 1) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (2187) - 1) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (2193) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (2193), acc);
    acc = (-rd(d, p + (11) - 1) / 4 + rd(d, p + (2184) - 1) / 2) as i16 as i32;
    wr(d, p + (2184), acc);
    acc = (acc * 3 / 2 + rd(d, p + (2193) - 1) + rd(d, p + (16381) - 13) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 13) + rd(d, p + (16366) - 83) / 2) as i16 as i32;
    wr(d, p + (16366), -acc);
    acc = (-acc / 2 + rd(d, p + (16366) - 83) + rd(d, p + (16281) - 116) / 2) as i16 as i32;
    wr(d, p + (16281), -acc);
    acc = (-acc / 2 + rd(d, p + (16281) - 116) + rd(d, p + (16163) - 239) / 2) as i16 as i32;
    wr(d, p + (16163), -acc);
    acc = (-acc / 2 + rd(d, p + (16163) - 239) + rd(d, p + (15922) - 339) / 2) as i16 as i32;
    wr(d, p + (15922), -acc);
    acc = (-acc / 2 + rd(d, p + (15922) - 339) + rd(d, p + (15581) - 481) / 2) as i16 as i32;
    wr(d, p + (15581), -acc);
    acc = (-acc / 2 + rd(d, p + (15581) - 481) + rd(d, p + (15098) - 555) / 2) as i16 as i32;
    wr(d, p + (15098), -acc);
    acc = (-acc / 2 + rd(d, p + (15098) - 555) + rd(d, p + (14541) - 823) / 2) as i16 as i32;
    wr(d, p + (14541), -acc);
    acc = (-acc / 2 + rd(d, p + (14541) - 823) + rd(d, p + (13716) - 999) / 2) as i16 as i32;
    wr(d, p + (13716), -acc);
    acc = (-acc / 2 + rd(d, p + (13716) - 999) + rd(d, p + (12715) - 1100) / 2) as i16 as i32;
    wr(d, p + (12715), -acc);
    acc = (-acc / 2 + rd(d, p + (12715) - 1100) + rd(d, p + (11613) - 1347) / 2) as i16 as i32;
    wr(d, p + (11613), -acc);
    acc = (-acc / 2 + rd(d, p + (11613) - 1347) + rd(d, p + (10264) - 1563) / 2) as i16 as i32;
    wr(d, p + (10264), -acc);
    acc = (-acc / 2 + rd(d, p + (10264) - 1563) + rd(d, p + (8699) - 1841) / 2) as i16 as i32;
    wr(d, p + (8699), -acc);
    acc = (-acc / 2 + rd(d, p + (8699) - 1841) + rd(d, p + (6856) - 2001) / 2) as i16 as i32;
    wr(d, p + (6856), -acc);
    acc = (-acc / 2 + rd(d, p + (6856) - 2001) + rd(d, p + (4853) - 2083) / 2) as i16 as i32;
    wr(d, p + (4853), -acc);
    acc = (-acc / 2 + rd(d, p + (4853) - 2083)) as i16 as i32;
    wr(d, p + (11), acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (2666) - 147) / 2) as i16 as i32;
    wr(d, p + (2666), -acc);
    acc = (-acc / 2 + rd(d, p + (2666) - 147) + rd(d, p + (2768) - 43) / 2) as i16 as i32;
    wr(d, p + (2768), -acc);
    acc = (-acc / 2 + rd(d, p + (2768) - 43) + rd(d, p + (2723) - 55) / 2) as i16 as i32;
    wr(d, p + (2723), -acc);
    acc = (-acc / 2 + rd(d, p + (2723) - 55)) as i16 as i32;
    wr(d, p + (2190), acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (2444) - 249) / 2) as i16 as i32;
    wr(d, p + (2444), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (2444) - 249) + rd(d, p + (2494) - 48) / 2) as i16 as i32;
    wr(d, p + (2494), -acc);
    acc = (-acc / 2 + rd(d, p + (2494) - 48) + rd(d, p + (2517) - 21) / 2) as i16 as i32;
    wr(d, p + (2517), -acc);
    acc = (-acc / 2 + rd(d, p + (2517) - 21)) as i16 as i32;
    wr(d, p + (2187), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_51(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12641) - 1065) * 3 / 64 + rd(d, p + (12641) - 1233) * 3 / 64 + rd(d, p + (12641) - 1555) * 3 / 64 + rd(d, p + (9021) - 269) * 3 / 64 + rd(d, p + (4536) - 2003) * -3 / 32 + rd(d, p + (9021) - 1891) * -3 / 32 + rd(d, p + (12641) - 21) * -3 / 32 + rd(d, p + (9021) - 532) * -3 / 32 + rd(d, p + (9021) - 872) * -3 / 32 + rd(d, p + (9021) - 1107) * -3 / 32 + rd(d, p + (9021) - 1414) * -3 / 32 + rd(d, p + (9021) - 1615) * -3 / 32 + rd(d, p + (9021) - 1914) * -3 / 32 + rd(d, p + (4536) - 173) * -3 / 32 + rd(d, p + (4536) - 460) * -3 / 32 + rd(d, p + (4536) - 780) * 3 / 16 + rd(d, p + (4536) - 1066) * 3 / 16 + rd(d, p + (4536) - 1344) * 3 / 16 + rd(d, p + (4536) - 1660) * 3 / 16 + rd(d, p + (4536) - 1941) * 3 / 16) as i16 as i32;
    wr(d, p + (4), -acc);
    acc = (-rd(d, p + (4536) - 2005) / 16 + -rd(d, p + (4536) - 1762) / 16 + -rd(d, p + (4536) - 1408) / 16 + -rd(d, p + (4536) - 1110) / 16 + rd(d, p + (4536) - 671) / 8 + rd(d, p + (9021) - 890) / 8 + rd(d, p + (12641) - 1010) / 8 + rd(d, p + (4536) - 907) / 8 + rd(d, p + (4536) - 595) / 8 + rd(d, p + (4536) - 342) / 8 + rd(d, p + (4536) - 41) / 8 + rd(d, p + (9021) - 1801) / 8 + rd(d, p + (9021) - 1509) / 8 + rd(d, p + (9021) - 1229) / 8 + rd(d, p + (9021) - 936) / 8 + -rd(d, p + (9021) - 637) / 4 + -rd(d, p + (9021) - 414) / 4 + -rd(d, p + (9021) - 105) / 4 + -rd(d, p + (12641) - 1434) / 4 + -rd(d, p + (12641) - 1119) / 4) as i16 as i32;
    wr(d, p + (6), -acc);
    acc = (rd(d, p + (0) - 1) * 81 / 32 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1992) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1992)) as i16 as i32;
    wr(d, p + (9021), acc);
    acc = (rd(d, p + (9021) - 2110) * 15 / 16 + rd(d, p + (6909) - 2371) / 2) as i16 as i32;
    wr(d, p + (6909), -acc);
    left = (rd(d, p + (6) - 1)) as i16;
    acc = (-acc / 2 + rd(d, p + (6909) - 2371)) as i16 as i32;
    wr(d, p + (4536), acc);
    acc = (rd(d, p + (4536) - 2157) / 2 + rd(d, p + (2377) - 2312) / 2) as i16 as i32;
    wr(d, p + (2377), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2377) - 2312) * 3 / 2) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (63) - 1) / 4 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (63), acc);
    right = (rd(d, p + (4) - 1)) as i16;
    acc = (rd(d, p + (63) - 0) + tmp_2) as i16 as i32;
    wr(d, p + (12641), acc);
    (left, right)
}

fn midifex_effect_52(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_2 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_7 = 0i32;
    let mut tmp_9 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12641) - 21) / 2 + rd(d, p + (9121) - 1891) / 2 + rd(d, p + (4636) - 2003) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 801) / 2 + rd(d, p + (9121) - 890) / 2 + rd(d, p + (4636) - 671) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14655) - 1620) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 1620) + rd(d, p + (13033) - 234) / 2) as i16 as i32;
    wr(d, p + (13033), -acc);
    acc = (-acc / 2 + rd(d, p + (13033) - 234) + rd(d, p + (12797) - 154) / 2) as i16 as i32;
    wr(d, p + (12797), -acc);
    acc = (-acc / 2 + rd(d, p + (12797) - 154)) as i16 as i32;
    tmp_2 = (acc) as i16 as i32;
    acc = (rd(d, p + (12641) - 1624) + rd(d, p + (11015) - 1892) / 2) as i16 as i32;
    wr(d, p + (11015), -acc);
    acc = (-acc / 2 + rd(d, p + (11015) - 1892)) as i16 as i32;
    wr(d, p + (9121), acc);
    acc = (rd(d, p + (9121) - 2110) * -27 / 32 + rd(d, p + (7009) - 2371) / 2) as i16 as i32;
    wr(d, p + (7009), -acc);
    acc = (-acc / 2 + rd(d, p + (7009) - 2371)) as i16 as i32;
    wr(d, p + (4636), acc);
    acc = (rd(d, p + (4636) - 2157) * 9 / 8 + rd(d, p + (2477) - 2312) / 2) as i16 as i32;
    wr(d, p + (2477), -acc);
    acc = (-acc / 2 + rd(d, p + (2477) - 2312)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (157) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (157), acc);
    acc = (rd(d, p + (157) - 0) + tmp_2) as i16 as i32;
    tmp_7 = (acc) as i16 as i32;
    acc = (rd(d, p + (163) - 1) * 15 / 16) as i16 as i32;
    tmp_9 = (acc) as i16 as i32;
    acc = (tmp_7 / 16 + tmp_9) as i16 as i32;
    wr(d, p + (163), acc);
    acc = (rd(d, p + (163) - 0)) as i16 as i32;
    tmp_9 = (-acc) as i16 as i32;
    acc = (tmp_7 + tmp_9) as i16 as i32;
    wr(d, p + (12641), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_53(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 153) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 153) + rd(d, p + (2120) - 273) / 2) as i16 as i32;
    wr(d, p + (2120), -acc);
    acc = (-acc / 2 + rd(d, p + (2120) - 273) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1845) - 32) / 2) as i16 as i32;
    wr(d, p + (1845), -acc);
    acc = (-acc / 2 + rd(d, p + (1845) - 32) + rd(d, p + (1811) - 175) / 2) as i16 as i32;
    wr(d, p + (1811), -acc);
    acc = (-acc / 2 + rd(d, p + (1811) - 175) + rd(d, p + (1634) - 233) / 2) as i16 as i32;
    wr(d, p + (1634), -acc);
    acc = (-acc / 2 + rd(d, p + (1634) - 233) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (acc * 3 / 32 + rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 1555) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 2046) / 32 + rd(d, p + (0) - 3569) / 32 + rd(d, p + (0) - 2256) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 3121) / 32 + rd(d, p + (0) - 2721) / 32 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 3036) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 3725) / 16 + -rd(d, p + (0) - 3889) / 16 + -rd(d, p + (0) - 4034) / 16 + -rd(d, p + (0) - 4152) / 16 + -rd(d, p + (0) - 4257) / 16 + -rd(d, p + (0) - 4492) / 16 + -rd(d, p + (0) - 4556) / 16 + -rd(d, p + (0) - 4727) / 16 + -rd(d, p + (0) - 4849) / 16 + -rd(d, p + (0) - 6035) / 16 + -rd(d, p + (0) - 5129) / 16 + -rd(d, p + (0) - 5235) / 16 + -rd(d, p + (0) - 6223) / 16 + -rd(d, p + (0) - 5534) / 16 + rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5034) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6446) / 8 + rd(d, p + (0) - 6534) / 8 + rd(d, p + (0) - 6782) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 7039) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 7221) / 8 + rd(d, p + (0) - 8565) / 8 + rd(d, p + (0) - 7534) / 8 + rd(d, p + (0) - 7736) / 8 + rd(d, p + (0) - 7891) / 8 + rd(d, p + (0) - 9215) / 8 + rd(d, p + (0) - 8146) / 8 + rd(d, p + (0) - 8278) / 8 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 8700) / 4 + -rd(d, p + (0) - 8885) / 4 + -rd(d, p + (0) - 9012) / 4 + -rd(d, p + (0) - 9171) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9449) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 9765) / 4 + -rd(d, p + (0) - 9867) / 4 + -rd(d, p + (0) - 10046) / 4 + -rd(d, p + (0) - 10110) / 4 + -rd(d, p + (0) - 10285) / 4 + -rd(d, p + (0) - 12074) / 4 + -rd(d, p + (0) - 10578) / 4 + -rd(d, p + (0) - 10734) / 4 + -rd(d, p + (0) - 11715) / 4 + -rd(d, p + (0) - 11023) / 4 + -rd(d, p + (0) - 11185) / 4 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2 + rd(d, p + (0) - 13217) / 2 + rd(d, p + (0) - 13378) / 2 + rd(d, p + (0) - 13534) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midifex_effect_54(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2295) - 21) / 2) as i16 as i32;
    wr(d, p + (2295), -acc);
    acc = (-acc / 2 + rd(d, p + (2295) - 21) + rd(d, p + (2272) - 153) / 2) as i16 as i32;
    wr(d, p + (2272), -acc);
    acc = (-acc / 2 + rd(d, p + (2272) - 153) + rd(d, p + (2117) - 273) / 2) as i16 as i32;
    wr(d, p + (2117), -acc);
    acc = (-acc / 2 + rd(d, p + (2117) - 273) + rd(d, p + (16381) - 12000) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1842) - 32) / 2) as i16 as i32;
    wr(d, p + (1842), -acc);
    acc = (-acc / 2 + rd(d, p + (1842) - 32) + rd(d, p + (1808) - 175) / 2) as i16 as i32;
    wr(d, p + (1808), -acc);
    acc = (-acc / 2 + rd(d, p + (1808) - 175) + rd(d, p + (1631) - 233) / 2) as i16 as i32;
    wr(d, p + (1631), -acc);
    acc = (-acc / 2 + rd(d, p + (1631) - 233) + rd(d, p + (16381) - 12000) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (acc * -3 / 64 + -rd(d, p + (16381) - 1065) / 64 + -rd(d, p + (16381) - 1119) / 64 + -rd(d, p + (16381) - 1233) / 64 + -rd(d, p + (16381) - 1434) / 64 + -rd(d, p + (16381) - 1555) / 64 + -rd(d, p + (16381) - 1742) / 64 + -rd(d, p + (16381) - 1862) / 64 + -rd(d, p + (16381) - 2046) / 64 + -rd(d, p + (16381) - 3569) / 64 + -rd(d, p + (16381) - 2256) / 64 + -rd(d, p + (16381) - 2443) / 64 + -rd(d, p + (16381) - 3121) / 64 + -rd(d, p + (16381) - 2721) / 64 + rd(d, p + (16381) - 2873) / 32 + rd(d, p + (16381) - 3036) / 32 + rd(d, p + (16381) - 2579) / 32 + rd(d, p + (16381) - 3267) / 32 + rd(d, p + (16381) - 3451) / 32 + rd(d, p + (16381) - 2149) / 32 + rd(d, p + (16381) - 3725) / 32 + rd(d, p + (16381) - 3889) / 32 + rd(d, p + (16381) - 4034) / 32 + rd(d, p + (16381) - 4152) / 32 + -rd(d, p + (16381) - 4257) / 16 + -rd(d, p + (16381) - 4492) / 16 + -rd(d, p + (16381) - 4556) / 16 + -rd(d, p + (16381) - 4727) / 16 + -rd(d, p + (16381) - 4849) / 16 + -rd(d, p + (16381) - 6035) / 16 + -rd(d, p + (16381) - 5129) / 16 + -rd(d, p + (16381) - 5235) / 16 + -rd(d, p + (16381) - 6223) / 16 + -rd(d, p + (16381) - 5534) / 16 + rd(d, p + (16381) - 5759) / 8 + rd(d, p + (16381) - 5837) / 8 + rd(d, p + (16381) - 5034) / 8 + rd(d, p + (16381) - 6159) / 8 + rd(d, p + (16381) - 5421) / 8 + rd(d, p + (16381) - 6446) / 8 + rd(d, p + (16381) - 6534) / 8 + rd(d, p + (16381) - 6782) / 8 + rd(d, p + (16381) - 6859) / 8 + rd(d, p + (16381) - 7039) / 8 + rd(d, p + (16381) - 7126) / 8 + rd(d, p + (16381) - 7221) / 8 + rd(d, p + (16381) - 8565) / 8 + rd(d, p + (16381) - 7534) / 8 + rd(d, p + (16381) - 7736) / 8 + rd(d, p + (16381) - 7891) / 8 + rd(d, p + (16381) - 9215) / 8 + rd(d, p + (16381) - 8146) / 8 + rd(d, p + (16381) - 8278) / 8 + -rd(d, p + (16381) - 8447) / 4 + -rd(d, p + (16381) - 7439) / 4 + -rd(d, p + (16381) - 8700) / 4 + -rd(d, p + (16381) - 8885) / 4 + -rd(d, p + (16381) - 9012) / 4 + -rd(d, p + (16381) - 9171) / 4 + -rd(d, p + (16381) - 8034) / 4 + -rd(d, p + (16381) - 9449) / 4 + -rd(d, p + (16381) - 9513) / 4 + -rd(d, p + (16381) - 9765) / 4 + -rd(d, p + (16381) - 9867) / 4 + -rd(d, p + (16381) - 10046) / 4 + -rd(d, p + (16381) - 10110) / 4 + -rd(d, p + (16381) - 10285) / 4 + -rd(d, p + (16381) - 12074) / 4 + -rd(d, p + (16381) - 10578) / 4 + -rd(d, p + (16381) - 10734) / 4 + -rd(d, p + (16381) - 11715) / 4 + -rd(d, p + (16381) - 11023) / 4 + -rd(d, p + (16381) - 11185) / 4 + rd(d, p + (16381) - 11254) / 2 + rd(d, p + (16381) - 11479) / 2 + rd(d, p + (16381) - 11546) / 2 + rd(d, p + (16381) - 10815) / 2 + rd(d, p + (16381) - 11847) / 2 + rd(d, p + (16381) - 10428) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    acc = (rd(d, p + (16381) - 14084) / 2 + rd(d, p + (0) - 1)) as i16 as i32;
    wr(d, p + (16381), acc);
    (left, right)
}

fn midifex_effect_55(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (0) - 8922) / 4 + rd(d, p + (0) - 5321) / 2 + rd(d, p + (0) - 4000) / 2 + rd(d, p + (0) - 2315) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (-rd(d, p + (0) - 9012) / 4 + rd(d, p + (0) - 6352) / 2 + rd(d, p + (0) - 4000) / 2 + rd(d, p + (0) - 3378) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_56(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 13998) / 128 + rd(d, p + (0) - 13968) / 128 + rd(d, p + (0) - 13955) / 128 + rd(d, p + (0) - 13957) / 128 + rd(d, p + (0) - 13815) / 128 + -rd(d, p + (0) - 13819) / 64 + -rd(d, p + (0) - 13749) / 64 + -rd(d, p + (0) - 13756) / 64 + -rd(d, p + (0) - 13676) / 64 + rd(d, p + (0) - 13552) / 32 + rd(d, p + (0) - 13319) / 32 + rd(d, p + (0) - 13185) / 32 + rd(d, p + (0) - 12927) / 32 + rd(d, p + (0) - 12656) / 32 + -rd(d, p + (0) - 12338) / 16 + -rd(d, p + (0) - 11919) / 16 + -rd(d, p + (0) - 11361) / 16 + -rd(d, p + (0) - 10683) / 16 + rd(d, p + (0) - 9835) / 8 + rd(d, p + (0) - 8775) / 8 + -rd(d, p + (0) - 9450) / 4 + -rd(d, p + (0) - 5194) / 4 + rd(d, p + (0) - 4501) / 2 + rd(d, p + (0) - 2724) / 2 + rd(d, p + (0) - 781) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 13986) / 128 + rd(d, p + (0) - 13919) / 128 + rd(d, p + (0) - 13918) / 128 + rd(d, p + (0) - 13972) / 128 + rd(d, p + (0) - 13807) / 128 + rd(d, p + (0) - 13854) / 128 + -rd(d, p + (0) - 13727) / 64 + -rd(d, p + (0) - 13619) / 64 + -rd(d, p + (0) - 13519) / 64 + rd(d, p + (0) - 13457) / 32 + rd(d, p + (0) - 13289) / 32 + rd(d, p + (0) - 13071) / 32 + -rd(d, p + (0) - 12727) / 16 + -rd(d, p + (0) - 12449) / 16 + -rd(d, p + (0) - 12016) / 16 + -rd(d, p + (0) - 11571) / 16 + rd(d, p + (0) - 10981) / 8 + rd(d, p + (0) - 10160) / 8 + rd(d, p + (0) - 9145) / 8 + rd(d, p + (0) - 8913) / 8 + -rd(d, p + (0) - 6873) / 4 + -rd(d, p + (0) - 4048) / 4 + rd(d, p + (0) - 3041) / 2 + rd(d, p + (0) - 1121) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_57(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12) - 1) / 2 + rd(d, p + (2298) - 67) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 67) + rd(d, p + (2229) - 121) / 2) as i16 as i32;
    wr(d, p + (2229), -acc);
    acc = (-acc / 2 + rd(d, p + (2229) - 121) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (10) - 1) / 2 + rd(d, p + (1931) - 45) / 2) as i16 as i32;
    wr(d, p + (1931), -acc);
    acc = (-acc / 2 + rd(d, p + (1931) - 45) + rd(d, p + (1884) - 98) / 2) as i16 as i32;
    wr(d, p + (1884), -acc);
    acc = (-acc / 2 + rd(d, p + (1884) - 98)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1555) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 3569) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 2721) / 32 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 13112) / 16 + -rd(d, p + (0) - 3725) / 16 + -rd(d, p + (0) - 3889) / 16 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 6035) / 8 + rd(d, p + (0) - 12345) / 8 + rd(d, p + (0) - 11111) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5534) / 8 + -rd(d, p + (0) - 5759) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 10211) / 4 + -rd(d, p + (0) - 9123) / 4 + -rd(d, p + (0) - 12781) / 4 + -rd(d, p + (0) - 5421) / 4 + -rd(d, p + (0) - 6534) / 4 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 9215) / 2 + rd(d, p + (0) - 8146) / 2) as i16 as i32;
    wr(d, p + (10), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5034) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6446) / 8 + rd(d, p + (0) - 6534) / 8 + rd(d, p + (0) - 6782) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 7039) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 7221) / 8 + rd(d, p + (0) - 8565) / 8 + rd(d, p + (0) - 7534) / 8 + rd(d, p + (0) - 7736) / 8 + rd(d, p + (0) - 7891) / 8 + rd(d, p + (0) - 9215) / 8 + rd(d, p + (0) - 8146) / 8 + rd(d, p + (0) - 8278) / 8 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 8700) / 4 + -rd(d, p + (0) - 8885) / 4 + -rd(d, p + (0) - 9012) / 4 + -rd(d, p + (0) - 9171) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9449) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 9765) / 4 + -rd(d, p + (0) - 9867) / 4 + -rd(d, p + (0) - 10046) / 4 + -rd(d, p + (0) - 10110) / 4 + -rd(d, p + (0) - 10285) / 4 + -rd(d, p + (0) - 12074) / 4 + -rd(d, p + (0) - 10578) / 4 + -rd(d, p + (0) - 10734) / 4 + -rd(d, p + (0) - 11715) / 4 + -rd(d, p + (0) - 11023) / 4 + -rd(d, p + (0) - 11185) / 4 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2 + rd(d, p + (0) - 13217) / 2 + rd(d, p + (0) - 13378) / 2 + rd(d, p + (0) - 13534) / 2) as i16 as i32;
    wr(d, p + (12), acc);
    (left, right)
}

fn midifex_effect_58(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_d = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7) + rd(d, p + (16375) - 43) / 2) as i16 as i32;
    wr(d, p + (16375), -acc);
    acc = (-acc / 2 + rd(d, p + (16375) - 43) + rd(d, p + (16330) - 50) / 2) as i16 as i32;
    wr(d, p + (16330), -acc);
    acc = (-acc / 2 + rd(d, p + (16330) - 50) + rd(d, p + (16278) - 63) / 2) as i16 as i32;
    wr(d, p + (16278), -acc);
    acc = (-acc / 2 + rd(d, p + (16278) - 63) + rd(d, p + (16213) - 75) / 2) as i16 as i32;
    wr(d, p + (16213), -acc);
    acc = (-acc / 2 + rd(d, p + (16213) - 75) + rd(d, p + (16136) - 87) / 2) as i16 as i32;
    wr(d, p + (16136), -acc);
    acc = (-acc / 2 + rd(d, p + (16136) - 87) + rd(d, p + (16047) - 102) / 2) as i16 as i32;
    wr(d, p + (16047), -acc);
    acc = (-acc / 2 + rd(d, p + (16047) - 102) + rd(d, p + (15943) - 126) / 2) as i16 as i32;
    wr(d, p + (15943), -acc);
    acc = (-acc / 2 + rd(d, p + (15943) - 126) + rd(d, p + (15815) - 149) / 2) as i16 as i32;
    wr(d, p + (15815), -acc);
    acc = (-acc / 2 + rd(d, p + (15815) - 149) + rd(d, p + (15664) - 179) / 2) as i16 as i32;
    wr(d, p + (15664), -acc);
    acc = (-acc / 2 + rd(d, p + (15664) - 179) + rd(d, p + (15483) - 210) / 2) as i16 as i32;
    wr(d, p + (15483), -acc);
    acc = (-acc / 2 + rd(d, p + (15483) - 210) + rd(d, p + (15271) - 257) / 2) as i16 as i32;
    wr(d, p + (15271), -acc);
    acc = (-acc / 2 + rd(d, p + (15271) - 257) + rd(d, p + (15012) - 308) / 2) as i16 as i32;
    wr(d, p + (15012), -acc);
    acc = (-acc / 2 + rd(d, p + (15012) - 308) + rd(d, p + (14702) - 363) / 2) as i16 as i32;
    wr(d, p + (14702), -acc);
    acc = (-acc / 2 + rd(d, p + (14702) - 363) + rd(d, p + (14337) - 439) / 2) as i16 as i32;
    wr(d, p + (14337), -acc);
    acc = (-acc / 2 + rd(d, p + (14337) - 439)) as i16 as i32;
    tmp_d = (acc) as i16 as i32;
    acc = (tmp_d + rd(d, p + (13896) - 35) / 2) as i16 as i32;
    wr(d, p + (13896), -acc);
    acc = (-acc / 2 + rd(d, p + (13896) - 35)) as i16 as i32;
    wr(d, p + (6), -acc);
    acc = (tmp_d + rd(d, p + (13859) - 31) / 2) as i16 as i32;
    wr(d, p + (13859), -acc);
    acc = (-acc / 2 + rd(d, p + (13859) - 31)) as i16 as i32;
    wr(d, p + (4), -acc);
    left = (rd(d, p + (6) - 1)) as i16;
    right = (rd(d, p + (4) - 1)) as i16;
    (left, right)
}

fn midifex_effect_59(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1) + rd(d, p + (5) - 1) / 2 + rd(d, p + (16381) - 34) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 34) + rd(d, p + (16345) - 45) / 2) as i16 as i32;
    wr(d, p + (16345), -acc);
    acc = (-acc / 2 + rd(d, p + (16345) - 45) + rd(d, p + (16298) - 67) / 2) as i16 as i32;
    wr(d, p + (16298), -acc);
    acc = (-acc / 2 + rd(d, p + (16298) - 67) + rd(d, p + (16229) - 89) / 2) as i16 as i32;
    wr(d, p + (16229), -acc);
    acc = (-acc / 2 + rd(d, p + (16229) - 89) + rd(d, p + (16138) - 101) / 2) as i16 as i32;
    wr(d, p + (16138), -acc);
    acc = (-acc / 2 + rd(d, p + (16138) - 101) + rd(d, p + (16035) - 120) / 2) as i16 as i32;
    wr(d, p + (16035), -acc);
    acc = (-acc / 2 + rd(d, p + (16035) - 120)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (3) - 0)) as i16 as i32;
    wr(d, p + (5), acc);
    left = (rd(d, p + (5) - 0)) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_60(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0 + rd(d, p + (16381) - 187) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 187) + rd(d, p + (16192) - 337) / 2) as i16 as i32;
    wr(d, p + (16192), -acc);
    acc = (-acc / 2 + rd(d, p + (16192) - 337)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (tmp_0 + rd(d, p + (15853) - 121) / 2) as i16 as i32;
    wr(d, p + (15853), -acc);
    acc = (-acc / 2 + rd(d, p + (15853) - 121) + rd(d, p + (15730) - 393) / 2) as i16 as i32;
    wr(d, p + (15730), -acc);
    acc = (-acc / 2 + rd(d, p + (15730) - 393)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_61(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_7d1 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1) + rd(d, p + (16381) - 47) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 47) + rd(d, p + (16332) - 89) / 2) as i16 as i32;
    wr(d, p + (16332), -acc);
    acc = (-acc / 2 + rd(d, p + (16332) - 89) + rd(d, p + (16241) - 215) / 2) as i16 as i32;
    wr(d, p + (16241), -acc);
    acc = (-acc / 2 + rd(d, p + (16241) - 215)) as i16 as i32;
    tmp_7d1 = (acc) as i16 as i32;
    acc = (tmp_7d1) as i16 as i32;
    wr(d, p + (301), acc);
    left = (rd(d, p + (301) - 296)) as i16;
    right = (rd(d, p + (301) - 298)) as i16;
    (left, right)
}

fn midifex_effect_62(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 150)) as i16 as i32;
    wr(d, p + (3), acc);
    acc = (rd(d, p + (0) - 300)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (rd(d, p + (3) - 0)) as i16;
    (left, right)
}

fn midifex_effect_63(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14261) - 1) / 4 + rd(d, p + (14264) - 1)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (-rd(d, p + (14264) - 1) / 4 + rd(d, p + (14261) - 1)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 3) * 3 / 2 + rd(d, p + (16089) - 15) / 2) as i16 as i32;
    wr(d, p + (16089), -acc);
    acc = (-acc / 2 + rd(d, p + (16089) - 15) + rd(d, p + (16072) - 22) / 2) as i16 as i32;
    wr(d, p + (16072), -acc);
    acc = (-acc / 2 + rd(d, p + (16072) - 22) + rd(d, p + (16048) - 29) / 2) as i16 as i32;
    wr(d, p + (16048), -acc);
    acc = (-acc / 2 + rd(d, p + (16048) - 29) + rd(d, p + (16017) - 45) / 2) as i16 as i32;
    wr(d, p + (16017), -acc);
    acc = (-acc / 2 + rd(d, p + (16017) - 45) + rd(d, p + (15970) - 61) / 2) as i16 as i32;
    wr(d, p + (15970), -acc);
    acc = (-acc / 2 + rd(d, p + (15970) - 61) + rd(d, p + (14997) - 105) / 2) as i16 as i32;
    wr(d, p + (14997), -acc);
    acc = (-acc / 2 + rd(d, p + (14997) - 105) + rd(d, p + (14890) - 149) / 2) as i16 as i32;
    wr(d, p + (14890), -acc);
    acc = (-acc / 2 + rd(d, p + (14890) - 149) + rd(d, p + (14739) - 213) / 2) as i16 as i32;
    wr(d, p + (14739), -acc);
    acc = (-acc / 2 + rd(d, p + (14739) - 213) + rd(d, p + (14524) - 258) / 2) as i16 as i32;
    wr(d, p + (14524), -acc);
    acc = (-acc / 2 + rd(d, p + (14524) - 258)) as i16 as i32;
    wr(d, p + (14264), acc);
    acc = (rd(d, p + (0) - 3) * 3 / 2 + rd(d, p + (15210) - 17) / 2) as i16 as i32;
    wr(d, p + (15210), -acc);
    acc = (-acc / 2 + rd(d, p + (15210) - 17) + rd(d, p + (15191) - 25) / 2) as i16 as i32;
    wr(d, p + (15191), -acc);
    acc = (-acc / 2 + rd(d, p + (15191) - 25) + rd(d, p + (15164) - 35) / 2) as i16 as i32;
    wr(d, p + (15164), -acc);
    acc = (-acc / 2 + rd(d, p + (15164) - 35) + rd(d, p + (15127) - 52) / 2) as i16 as i32;
    wr(d, p + (15127), -acc);
    acc = (-acc / 2 + rd(d, p + (15127) - 52) + rd(d, p + (15073) - 74) / 2) as i16 as i32;
    wr(d, p + (15073), -acc);
    acc = (-acc / 2 + rd(d, p + (15073) - 74) + rd(d, p + (15907) - 88) / 2) as i16 as i32;
    wr(d, p + (15907), -acc);
    acc = (-acc / 2 + rd(d, p + (15907) - 88) + rd(d, p + (15817) - 123) / 2) as i16 as i32;
    wr(d, p + (15817), -acc);
    acc = (-acc / 2 + rd(d, p + (15817) - 123) + rd(d, p + (15692) - 174) / 2) as i16 as i32;
    wr(d, p + (15692), -acc);
    acc = (-acc / 2 + rd(d, p + (15692) - 174) + rd(d, p + (15516) - 304) / 2) as i16 as i32;
    wr(d, p + (15516), -acc);
    acc = (-acc / 2 + rd(d, p + (15516) - 304)) as i16 as i32;
    wr(d, p + (14261), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midifex_effect_64(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    left = (rd(d, p + (140) - 135)) as i16;
    right = (rd(d, p + (140) - 137)) as i16;
    acc = (0) as i16 as i32;
    wr(d, p + (140), -acc);
    (left, right)
}

pub static MODEL_FX: [Program; 64] = [
    Program { name: "ECHO LONG FLAT AMBI", run: midifex_effect_1 },
    Program { name: "ECHO LONG FLAT THICK", run: midifex_effect_2 },
    Program { name: "ECHO LONG HPF", run: midifex_effect_3 },
    Program { name: "ECHO LONG HPF WIDE", run: midifex_effect_4 },
    Program { name: "ECHO LONG BPF AMBI", run: midifex_effect_5 },
    Program { name: "ECHO LONG LPF WIDE", run: midifex_effect_6 },
    Program { name: "ECHO MED FLAT AMBI", run: midifex_effect_7 },
    Program { name: "ECHO MED FLAT WIDE", run: midifex_effect_8 },
    Program { name: "ECHO MED HPF AMBI", run: midifex_effect_9 },
    Program { name: "ECHO MED BPF AMBI", run: midifex_effect_10 },
    Program { name: "ECHO MED LPF AMBI", run: midifex_effect_11 },
    Program { name: "ECHO MED LPF WIDE", run: midifex_effect_12 },
    Program { name: "ECHO MED FLAT THICK", run: midifex_effect_13 },
    Program { name: "ECHO SHORT FLAT", run: midifex_effect_14 },
    Program { name: "ECHO SHORT LPF AMBI", run: midifex_effect_15 },
    Program { name: "ECHO SHORT HPF AMBI", run: midifex_effect_16 },
    Program { name: "ECHO SHORT BPF AMBI", run: midifex_effect_17 },
    Program { name: "ECHO SHORT FLAT", run: midifex_effect_18 },
    Program { name: "ECHO SHORT FLAT WIDE", run: midifex_effect_19 },
    Program { name: "ECHO XSHORT FLAT", run: midifex_effect_20 },
    Program { name: "ECHO XSHORT BPF WIDE", run: midifex_effect_21 },
    Program { name: "2TAP MED FLAT AMBI", run: midifex_effect_22 },
    Program { name: "2TAP MED HPF AMBI", run: midifex_effect_23 },
    Program { name: "2TAP MED BPF AMBI", run: midifex_effect_24 },
    Program { name: "2TAP MED FLAT THICK", run: midifex_effect_25 },
    Program { name: "2TAP SHORT FLAT WIDE", run: midifex_effect_26 },
    Program { name: "2TAP SHORT HPF PAN", run: midifex_effect_27 },
    Program { name: "2TAP SHORT BPF AMBI", run: midifex_effect_28 },
    Program { name: "2TAP SHORT LPF AMBI", run: midifex_effect_29 },
    Program { name: "2TAP XSHORT FLAT WIDE", run: midifex_effect_30 },
    Program { name: "3TAP MED FLAT PAN", run: midifex_effect_31 },
    Program { name: "3TAP SHORT FLAT PAN", run: midifex_effect_32 },
    Program { name: "3TAP SHORT LPF AMBI", run: midifex_effect_33 },
    Program { name: "3TAP SHORT BPF AMBI", run: midifex_effect_34 },
    Program { name: "3TAP SHORT HPF AMBI", run: midifex_effect_35 },
    Program { name: "3TAP XSHORT FLAT AMBI", run: midifex_effect_36 },
    Program { name: "REGEN MED FLAT", run: midifex_effect_37 },
    Program { name: "REGEN MED HPF AMBI", run: midifex_effect_38 },
    Program { name: "REGEN MED BPF AMBI", run: midifex_effect_39 },
    Program { name: "REGEN MED LPF AMBI", run: midifex_effect_40 },
    Program { name: "REGEN SHORT FLAT", run: midifex_effect_41 },
    Program { name: "REGEN XSHORT FLAT", run: midifex_effect_42 },
    Program { name: "SLAP1", run: midifex_effect_43 },
    Program { name: "SLAP2", run: midifex_effect_44 },
    Program { name: "SLAP3", run: midifex_effect_45 },
    Program { name: "SLAP4", run: midifex_effect_46 },
    Program { name: "SLAP5", run: midifex_effect_47 },
    Program { name: "REVERB SHORT GATE", run: midifex_effect_48 },
    Program { name: "REVERB MEDIUM WARM", run: midifex_effect_49 },
    Program { name: "REVERB MEDIUM BLOOM", run: midifex_effect_50 },
    Program { name: "REVERB MEDIUM PAN", run: midifex_effect_51 },
    Program { name: "REVERB LONG HPF", run: midifex_effect_52 },
    Program { name: "REVERB REVERSE", run: midifex_effect_53 },
    Program { name: "REVERB REVERSE REGEN", run: midifex_effect_54 },
    Program { name: "MULTITAP PAN", run: midifex_effect_55 },
    Program { name: "MULTITAP REVERB", run: midifex_effect_56 },
    Program { name: "MULTITAP REVERSE PAN", run: midifex_effect_57 },
    Program { name: "THICKENER", run: midifex_effect_58 },
    Program { name: "THICKENER DENSE", run: midifex_effect_59 },
    Program { name: "STEREOGEN AMBIENT", run: midifex_effect_60 },
    Program { name: "STEREOGEN THICK", run: midifex_effect_61 },
    Program { name: "STEREOGEN WIDE", run: midifex_effect_62 },
    Program { name: "STEREOGEN XWIDE", run: midifex_effect_63 },
    Program { name: "DEFEAT", run: midifex_effect_64 },
];

fn midiverb2_effect_0(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    left = (rd(d, p + (140) - 135)) as i16;
    right = (rd(d, p + (140) - 137)) as i16;
    acc = (0) as i16 as i32;
    wr(d, p + (140), -acc);
    (left, right)
}

fn midiverb2_effect_1(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15972) - 12) * 3 / 4 + rd(d, p + (15549) - 234) * 3 / 4 + rd(d, p + (15035) - 252) * 3 / 4 + rd(d, p + (14477) - 378) * 3 / 4 + rd(d, p + (12812) - 169) * 3 / 4 + rd(d, p + (12375) - 199) * 3 / 4 + rd(d, p + (11892) - 423) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15972) - 145) * 3 / 4 + rd(d, p + (15549) - 130) * 3 / 4 + rd(d, p + (15035) - 459) * 3 / 4 + rd(d, p + (14477) - 277) * 3 / 4 + rd(d, p + (12812) - 103) * 3 / 4 + rd(d, p + (12375) - 208) * 3 / 4 + rd(d, p + (10744) - 16) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 115) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 136) * 3 / 4 + rd(d, p + (16227) - 115)) as i16 as i32;
    wr(d, p + (16110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (16110) - 136) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15972) - 421) / 2 + rd(d, p + (13797) - 278) / 2) as i16 as i32;
    wr(d, p + (13797), -acc);
    acc = (-acc / 2 + rd(d, p + (13797) - 278)) as i16 as i32;
    wr(d, p + (15549), acc);
    acc = (-rd(d, p + (15549) - 512) / 4 + -rd(d, p + (15549) - 513) / 4 + rd(d, p + (13517) - 212) / 2) as i16 as i32;
    wr(d, p + (13517), -acc);
    acc = (-acc / 2 + rd(d, p + (13517) - 212)) as i16 as i32;
    wr(d, p + (15035), acc);
    acc = (rd(d, p + (15035) - 556) / 2 + rd(d, p + (13303) - 256) / 2) as i16 as i32;
    wr(d, p + (13303), -acc);
    acc = (-acc / 2 + rd(d, p + (13303) - 256)) as i16 as i32;
    wr(d, p + (14477), acc);
    acc = (rd(d, p + (14477) - 678) / 2 + rd(d, p + (13045) - 231) / 2) as i16 as i32;
    wr(d, p + (13045), -acc);
    acc = (-acc / 2 + rd(d, p + (13045) - 231) + rd(d, p + (9694) - 1) / 2) as i16 as i32;
    wr(d, p + (9694), acc);
    acc = (rd(d, p + (9694) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (15972), acc);
    acc = (rd(d, p + (12812) - 435) / 2 + rd(d, p + (10744) - 292) / 2) as i16 as i32;
    wr(d, p + (10744), -acc);
    acc = (-acc / 2 + rd(d, p + (10744) - 292)) as i16 as i32;
    wr(d, p + (12375), acc);
    acc = (-rd(d, p + (12375) - 481) / 4 + -rd(d, p + (12375) - 482) / 4 + rd(d, p + (10450) - 281) / 2) as i16 as i32;
    wr(d, p + (10450), -acc);
    acc = (-acc / 2 + rd(d, p + (10450) - 281)) as i16 as i32;
    wr(d, p + (11892), acc);
    acc = (rd(d, p + (11892) - 571) / 2 + rd(d, p + (10167) - 234) / 2) as i16 as i32;
    wr(d, p + (10167), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (10167) - 234)) as i16 as i32;
    wr(d, p + (11319), acc);
    acc = (rd(d, p + (11319) - 573) / 2 + rd(d, p + (9931) - 232) / 2) as i16 as i32;
    wr(d, p + (9931), -acc);
    acc = (-acc / 2 + rd(d, p + (9931) - 232) + rd(d, p + (9691) - 1) / 2) as i16 as i32;
    wr(d, p + (9691), acc);
    acc = (rd(d, p + (9691) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (12812), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_2(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15962) - 120) * 3 / 4 + rd(d, p + (15092) - 334) * 3 / 4 + rd(d, p + (12751) - 252) * 3 / 4 + rd(d, p + (11359) - 478) * 3 / 4 + rd(d, p + (9457) - 369) * 3 / 4 + rd(d, p + (8482) - 499) * 3 / 4 + rd(d, p + (6402) - 723) * 3 / 4 + rd(d, p + (5629) - 214) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15962) - 129) * 3 / 4 + rd(d, p + (15092) - 130) * 3 / 4 + rd(d, p + (12751) - 459) * 3 / 4 + rd(d, p + (11359) - 277) * 3 / 4 + rd(d, p + (9457) - 363) * 3 / 4 + rd(d, p + (8482) - 208) * 3 / 4 + rd(d, p + (6402) - 421) * 3 / 4 + rd(d, p + (5629) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 115) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 146) * 3 / 4 + rd(d, p + (16227) - 115)) as i16 as i32;
    wr(d, p + (16110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (16110) - 146) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15962) - 868) / 2 + rd(d, p + (13745) - 678) / 2) as i16 as i32;
    wr(d, p + (13745), -acc);
    acc = (-acc / 2 + rd(d, p + (13745) - 678)) as i16 as i32;
    wr(d, p + (15092), acc);
    acc = (rd(d, p + (15092) - 1345) / 2 + rd(d, p + (13065) - 312) / 2) as i16 as i32;
    wr(d, p + (13065), -acc);
    acc = (-acc / 2 + rd(d, p + (13065) - 312) + rd(d, p + (3636) - 1) / 2) as i16 as i32;
    wr(d, p + (3636), acc);
    acc = (rd(d, p + (3636) - 0) / 2 + tmp_b / 2) as i16 as i32;
    wr(d, p + (15962), acc);
    acc = (rd(d, p + (12751) - 1390) / 2 + rd(d, p + (10568) - 676) / 2) as i16 as i32;
    wr(d, p + (10568), -acc);
    acc = (-acc / 2 + rd(d, p + (10568) - 676)) as i16 as i32;
    wr(d, p + (11359), acc);
    acc = (rd(d, p + (11359) - 789) / 2 + rd(d, p + (9890) - 431) / 2) as i16 as i32;
    wr(d, p + (9890), -acc);
    acc = (-acc / 2 + rd(d, p + (9890) - 431) + rd(d, p + (3633) - 1) / 2) as i16 as i32;
    wr(d, p + (3633), acc);
    acc = (rd(d, p + (3633) - 0) / 2 + tmp_b / 2) as i16 as i32;
    wr(d, p + (12751), acc);
    acc = (rd(d, p + (9457) - 973) / 2 + rd(d, p + (7499) - 712) / 2) as i16 as i32;
    wr(d, p + (7499), -acc);
    acc = (-acc / 2 + rd(d, p + (7499) - 712)) as i16 as i32;
    wr(d, p + (8482), acc);
    acc = (rd(d, p + (8482) - 981) / 2 + rd(d, p + (6785) - 381) / 2) as i16 as i32;
    wr(d, p + (6785), -acc);
    acc = (-acc / 2 + rd(d, p + (6785) - 381) + rd(d, p + (3630) - 1) / 2) as i16 as i32;
    wr(d, p + (3630), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3630) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (9457), acc);
    acc = (rd(d, p + (6402) - 771) / 2 + rd(d, p + (4606) - 634) / 2) as i16 as i32;
    wr(d, p + (4606), -acc);
    acc = (-acc / 2 + rd(d, p + (4606) - 634)) as i16 as i32;
    wr(d, p + (5629), acc);
    acc = (rd(d, p + (5629) - 1021) / 2 + rd(d, p + (3970) - 332) / 2) as i16 as i32;
    wr(d, p + (3970), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (3970) - 332) + rd(d, p + (3627) - 1) / 2) as i16 as i32;
    wr(d, p + (3627), acc);
    acc = (rd(d, p + (3627) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (6402), acc);
    (left, right)
}

fn midiverb2_effect_3(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15962) - 320) * 3 / 4 + rd(d, p + (15092) - 734) * 3 / 4 + rd(d, p + (12551) - 252) * 3 / 4 + rd(d, p + (11159) - 478) * 3 / 4 + rd(d, p + (8857) - 369) * 3 / 4 + rd(d, p + (7621) - 499) * 3 / 4 + rd(d, p + (5441) - 723) * 3 / 4 + rd(d, p + (4468) - 214) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15962) - 129) * 3 / 4 + rd(d, p + (15092) - 130) * 3 / 4 + rd(d, p + (12551) - 859) * 3 / 4 + rd(d, p + (11159) - 277) * 3 / 4 + rd(d, p + (8857) - 563) * 3 / 4 + rd(d, p + (7621) - 208) * 3 / 4 + rd(d, p + (5441) - 421) * 3 / 4 + rd(d, p + (4468) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 115) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 146) * 3 / 4 + rd(d, p + (16227) - 115)) as i16 as i32;
    wr(d, p + (16110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (16110) - 146) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15962) - 868) / 2 + rd(d, p + (13745) - 678) / 2) as i16 as i32;
    wr(d, p + (13745), -acc);
    acc = (-acc / 2 + rd(d, p + (13745) - 678)) as i16 as i32;
    wr(d, p + (15092), acc);
    acc = (rd(d, p + (15092) - 1345) / 2 + rd(d, p + (13065) - 512) / 2) as i16 as i32;
    wr(d, p + (13065), -acc);
    acc = (-acc / 2 + rd(d, p + (13065) - 512) + rd(d, p + (2475) - 1) / 2) as i16 as i32;
    wr(d, p + (2475), acc);
    acc = (rd(d, p + (2475) - 0) / 2 + tmp_b / 2) as i16 as i32;
    wr(d, p + (15962), acc);
    acc = (rd(d, p + (12551) - 1390) / 2 + rd(d, p + (10168) - 676) / 2) as i16 as i32;
    wr(d, p + (10168), -acc);
    acc = (-acc / 2 + rd(d, p + (10168) - 676)) as i16 as i32;
    wr(d, p + (11159), acc);
    acc = (rd(d, p + (11159) - 989) / 2 + rd(d, p + (9490) - 631) / 2) as i16 as i32;
    wr(d, p + (9490), -acc);
    acc = (-acc / 2 + rd(d, p + (9490) - 631) + rd(d, p + (2472) - 1) / 2) as i16 as i32;
    wr(d, p + (2472), acc);
    acc = (rd(d, p + (2472) - 0) / 2 + tmp_b / 2) as i16 as i32;
    wr(d, p + (12551), acc);
    acc = (rd(d, p + (8857) - 1234) / 2 + rd(d, p + (6638) - 712) / 2) as i16 as i32;
    wr(d, p + (6638), -acc);
    acc = (-acc / 2 + rd(d, p + (6638) - 712)) as i16 as i32;
    wr(d, p + (7621), acc);
    acc = (rd(d, p + (7621) - 981) / 2 + rd(d, p + (5924) - 481) / 2) as i16 as i32;
    wr(d, p + (5924), -acc);
    acc = (-acc / 2 + rd(d, p + (5924) - 481) + rd(d, p + (2469) - 1) / 2) as i16 as i32;
    wr(d, p + (2469), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (2469) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (8857), acc);
    acc = (rd(d, p + (5441) - 971) / 2 + rd(d, p + (3445) - 634) / 2) as i16 as i32;
    wr(d, p + (3445), -acc);
    acc = (-acc / 2 + rd(d, p + (3445) - 634)) as i16 as i32;
    wr(d, p + (4468), acc);
    acc = (rd(d, p + (4468) - 1021) / 2 + rd(d, p + (2809) - 332) / 2) as i16 as i32;
    wr(d, p + (2809), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (2809) - 332) + rd(d, p + (2466) - 1) / 2) as i16 as i32;
    wr(d, p + (2466), acc);
    acc = (rd(d, p + (2466) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (5441), acc);
    (left, right)
}

fn midiverb2_effect_4(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (14885) - 361) * 9 / 8 + rd(d, p + (11865) - 789) * 9 / 8 + rd(d, p + (7880) - 1703) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (14885) - 576) * 9 / 8 + rd(d, p + (11865) - 390) * 9 / 8 + rd(d, p + (7880) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (3607) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (3607), acc);
    acc = (rd(d, p + (3607) - 0) / 2 + rd(d, p + (16381) - 36) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 36) + rd(d, p + (16343) - 79) / 2) as i16 as i32;
    wr(d, p + (16343), -acc);
    acc = (-acc / 2 + rd(d, p + (16343) - 79) + rd(d, p + (16262) - 158) / 2) as i16 as i32;
    wr(d, p + (16262), -acc);
    acc = (-acc / 2 + rd(d, p + (16262) - 158) + rd(d, p + (16102) - 269) / 2) as i16 as i32;
    wr(d, p + (16102), -acc);
    acc = (-acc / 2 + rd(d, p + (16102) - 269) + rd(d, p + (15831) - 344) / 2) as i16 as i32;
    wr(d, p + (15831), -acc);
    acc = (-acc / 2 + rd(d, p + (15831) - 344) + rd(d, p + (15485) - 598) / 2) as i16 as i32;
    wr(d, p + (15485), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15485) - 598) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (14885) - 1624) / 2 + rd(d, p + (14885) - 1625) / 2 + rd(d, p + (13259) - 1392) / 2) as i16 as i32;
    wr(d, p + (13259), -acc);
    acc = (-acc / 2 + rd(d, p + (13259) - 1392)) as i16 as i32;
    wr(d, p + (11865), acc);
    acc = (-rd(d, p + (11865) - 2410) / 4 + rd(d, p + (9453) - 1571) / 2) as i16 as i32;
    wr(d, p + (9453), -acc);
    acc = (-acc / 2 + rd(d, p + (9453) - 1571)) as i16 as i32;
    wr(d, p + (7880), acc);
    acc = (rd(d, p + (7880) - 2657) * 3 / 4 + rd(d, p + (5221) - 1612) / 2) as i16 as i32;
    wr(d, p + (5221), -acc);
    acc = (-acc / 2 + rd(d, p + (5221) - 1612) + rd(d, p + (3604) - 1) / 2) as i16 as i32;
    wr(d, p + (3604), acc);
    acc = (rd(d, p + (3604) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (14885), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_5(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15952) - 120) * 9 / 8 + rd(d, p + (15282) - 334) * 9 / 8 + rd(d, p + (14568) - 252) * 9 / 8 + rd(d, p + (13710) - 478) * 9 / 8 + rd(d, p + (11245) - 369) * 9 / 8 + rd(d, p + (10670) - 499) * 9 / 8 + rd(d, p + (9987) - 723) * 9 / 8 + rd(d, p + (9114) - 207) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15952) - 129) * 9 / 8 + rd(d, p + (15282) - 130) * 9 / 8 + rd(d, p + (14568) - 459) * 9 / 8 + rd(d, p + (13710) - 277) * 9 / 8 + rd(d, p + (11245) - 363) * 9 / 8 + rd(d, p + (10670) - 208) * 9 / 8 + rd(d, p + (9987) - 451) * 9 / 8 + rd(d, p + (8279) - 56) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (6812) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (6812), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 115) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 156) * 3 / 4 + rd(d, p + (16227) - 115)) as i16 as i32;
    wr(d, p + (16110), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 156)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15952) - 668) * 3 / 4 + rd(d, p + (12830) - 478) / 2) as i16 as i32;
    wr(d, p + (12830), -acc);
    acc = (-acc / 2 + rd(d, p + (12830) - 478)) as i16 as i32;
    wr(d, p + (15282), acc);
    acc = (rd(d, p + (15282) - 712) * 3 / 4 + rd(d, p + (12350) - 312) / 2) as i16 as i32;
    wr(d, p + (12350), -acc);
    acc = (-acc / 2 + rd(d, p + (12350) - 312)) as i16 as i32;
    wr(d, p + (14568), acc);
    acc = (rd(d, p + (14568) - 856) * 3 / 4 + rd(d, p + (12036) - 456) / 2) as i16 as i32;
    wr(d, p + (12036), -acc);
    acc = (-acc / 2 + rd(d, p + (12036) - 456)) as i16 as i32;
    wr(d, p + (13710), acc);
    acc = (rd(d, p + (13710) - 878) * 3 / 4 + rd(d, p + (11578) - 331) / 2) as i16 as i32;
    wr(d, p + (11578), -acc);
    acc = (-acc / 2 + rd(d, p + (11578) - 331) + rd(d, p + (6809) - 1) / 2) as i16 as i32;
    wr(d, p + (6809), acc);
    acc = (rd(d, p + (6809) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (15952), acc);
    acc = (rd(d, p + (11245) - 573) * 3 / 4 + rd(d, p + (8279) - 412) / 2) as i16 as i32;
    wr(d, p + (8279), -acc);
    acc = (-acc / 2 + rd(d, p + (8279) - 412)) as i16 as i32;
    wr(d, p + (10670), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (10670) - 681) * 3 / 4 + rd(d, p + (7865) - 381) / 2) as i16 as i32;
    wr(d, p + (7865), -acc);
    acc = (-acc / 2 + rd(d, p + (7865) - 381)) as i16 as i32;
    wr(d, p + (9987), acc);
    acc = (rd(d, p + (9987) - 871) * 3 / 4 + rd(d, p + (7482) - 434) / 2) as i16 as i32;
    wr(d, p + (7482), -acc);
    acc = (-acc / 2 + rd(d, p + (7482) - 434)) as i16 as i32;
    wr(d, p + (9114), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (9114) - 833) * 3 / 4 + rd(d, p + (7046) - 232) / 2) as i16 as i32;
    wr(d, p + (7046), -acc);
    acc = (-acc / 2 + rd(d, p + (7046) - 232) + rd(d, p + (6806) - 1) / 2) as i16 as i32;
    wr(d, p + (6806), acc);
    acc = (rd(d, p + (6806) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (11245), acc);
    (left, right)
}

fn midiverb2_effect_6(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (14666) - 345) * 9 / 8 + rd(d, p + (10834) - 987) * 9 / 8 + rd(d, p + (3735) - 678) * 9 / 8 + rd(d, p + (7372) - 345) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (14666) - 685) * 9 / 8 + rd(d, p + (10834) - 342) * 9 / 8 + rd(d, p + (3735) - 340) * 9 / 8 + rd(d, p + (7372) - 1023) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (0) - 100) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (-acc / 8 + rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) / 2 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (-acc / 8 + rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) / 2 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (-acc / 8 + rd(d, p + (15613) - 598) * 5 / 8 + rd(d, p + (15982) - 0) / 2 + rd(d, p + (15982) - 367)) as i16 as i32;
    wr(d, p + (15613), -acc);
    acc = (-acc / 8 + rd(d, p + (15613) - 0) / 2 + rd(d, p + (15613) - 598) + rd(d, p + (15013) - 345) / 2) as i16 as i32;
    wr(d, p + (15013), -acc);
    acc = (-acc / 2 + rd(d, p + (15013) - 345)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (14666) - 2203) / 2 + rd(d, p + (14666) - 2204) / 2 + rd(d, p + (12461) - 1625) / 2) as i16 as i32;
    wr(d, p + (12461), -acc);
    acc = (-acc / 2 + rd(d, p + (12461) - 1625)) as i16 as i32;
    wr(d, p + (10834), acc);
    acc = (-rd(d, p + (10834) - 1969) / 4 + rd(d, p + (8863) - 1489) / 2) as i16 as i32;
    wr(d, p + (8863), -acc);
    acc = (-acc / 2 + rd(d, p + (8863) - 1489) + rd(d, p + (160) - 1) / 2) as i16 as i32;
    wr(d, p + (160), acc);
    acc = (rd(d, p + (160) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (14666), acc);
    acc = (rd(d, p + (7372) - 1934) / 2 + rd(d, p + (7372) - 1935) / 2 + rd(d, p + (5436) - 1699) / 2) as i16 as i32;
    wr(d, p + (5436), -acc);
    acc = (-acc / 2 + rd(d, p + (5436) - 1699)) as i16 as i32;
    wr(d, p + (3735), acc);
    acc = (-rd(d, p + (3735) - 2183) / 4 + rd(d, p + (1550) - 1385) / 2) as i16 as i32;
    wr(d, p + (1550), -acc);
    acc = (-acc / 2 + rd(d, p + (1550) - 1385) + rd(d, p + (157) - 1) / 2) as i16 as i32;
    wr(d, p + (157), acc);
    acc = (rd(d, p + (157) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (7372), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_7(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (14666) - 1209) * 9 / 8 + rd(d, p + (10834) - 987) * 9 / 8 + rd(d, p + (3735) - 678) * 9 / 8 + rd(d, p + (7372) - 787) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (14666) - 685) * 9 / 8 + rd(d, p + (10834) - 342) * 9 / 8 + rd(d, p + (3735) - 340) * 9 / 8 + rd(d, p + (7372) - 1023) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 100) / 2 + rd(d, p + (163) - 1) / 2) as i16 as i32;
    wr(d, p + (163), acc);
    acc = (rd(d, p + (16282) - 98) * 5 / 8 + rd(d, p + (163) - 1) / 2) as i16 as i32;
    wr(d, p + (16282), -acc);
    acc = (-acc / 8 + rd(d, p + (16182) - 198) * 5 / 8 + rd(d, p + (16282) - 0) / 2 + rd(d, p + (16282) - 98)) as i16 as i32;
    wr(d, p + (16182), -acc);
    acc = (-acc / 8 + rd(d, p + (15982) - 367) * 5 / 8 + rd(d, p + (16182) - 0) / 2 + rd(d, p + (16182) - 198)) as i16 as i32;
    wr(d, p + (15982), -acc);
    acc = (-acc / 8 + rd(d, p + (15613) - 598) * 5 / 8 + rd(d, p + (15982) - 0) / 2 + rd(d, p + (15982) - 367)) as i16 as i32;
    wr(d, p + (15613), -acc);
    acc = (-acc / 8 + rd(d, p + (15613) - 0) / 2 + rd(d, p + (15613) - 598) + rd(d, p + (15013) - 345) / 2) as i16 as i32;
    wr(d, p + (15013), -acc);
    acc = (-acc / 2 + rd(d, p + (15013) - 345)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (14666) - 2203) / 2 + rd(d, p + (14666) - 2204) / 2 + rd(d, p + (12461) - 1625) / 2) as i16 as i32;
    wr(d, p + (12461), -acc);
    acc = (-acc / 2 + rd(d, p + (12461) - 1625)) as i16 as i32;
    wr(d, p + (10834), acc);
    acc = (-rd(d, p + (10834) - 1969) / 4 + rd(d, p + (8863) - 1489) / 2) as i16 as i32;
    wr(d, p + (8863), -acc);
    acc = (-acc / 2 + rd(d, p + (8863) - 1489) + rd(d, p + (160) - 1) / 2) as i16 as i32;
    wr(d, p + (160), acc);
    acc = (rd(d, p + (160) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (14666), acc);
    acc = (rd(d, p + (7372) - 1934) / 2 + rd(d, p + (7372) - 1935) / 2 + rd(d, p + (5436) - 1699) / 2) as i16 as i32;
    wr(d, p + (5436), -acc);
    acc = (-acc / 2 + rd(d, p + (5436) - 1699)) as i16 as i32;
    wr(d, p + (3735), acc);
    acc = (-rd(d, p + (3735) - 2183) / 4 + rd(d, p + (1550) - 1385) / 2) as i16 as i32;
    wr(d, p + (1550), -acc);
    acc = (-acc / 2 + rd(d, p + (1550) - 1385) + rd(d, p + (157) - 1) / 2) as i16 as i32;
    wr(d, p + (157), acc);
    acc = (rd(d, p + (157) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (7372), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_8(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15952) - 120) * 9 / 8 + rd(d, p + (15282) - 334) * 9 / 8 + rd(d, p + (14568) - 252) * 9 / 8 + rd(d, p + (13710) - 478) * 9 / 8 + rd(d, p + (11245) - 369) * 9 / 8 + rd(d, p + (10670) - 499) * 9 / 8 + rd(d, p + (9987) - 723) * 9 / 8 + rd(d, p + (9114) - 207) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15952) - 129) * 9 / 8 + rd(d, p + (15282) - 130) * 9 / 8 + rd(d, p + (14568) - 459) * 9 / 8 + rd(d, p + (13710) - 277) * 9 / 8 + rd(d, p + (11245) - 363) * 9 / 8 + rd(d, p + (10670) - 208) * 9 / 8 + rd(d, p + (9987) - 451) * 9 / 8 + rd(d, p + (8279) - 56) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (6812) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (6812), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (6812) - 0) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 115) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 156) * 3 / 4 + rd(d, p + (16227) - 115)) as i16 as i32;
    wr(d, p + (16110), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16110) - 156)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15952) - 668) * 3 / 4 + rd(d, p + (12830) - 478) / 2) as i16 as i32;
    wr(d, p + (12830), -acc);
    acc = (-acc / 2 + rd(d, p + (12830) - 478)) as i16 as i32;
    wr(d, p + (15282), acc);
    acc = (rd(d, p + (15282) - 712) * 3 / 4 + rd(d, p + (12350) - 312) / 2) as i16 as i32;
    wr(d, p + (12350), -acc);
    acc = (-acc / 2 + rd(d, p + (12350) - 312)) as i16 as i32;
    wr(d, p + (14568), acc);
    acc = (rd(d, p + (14568) - 856) * 3 / 4 + rd(d, p + (12036) - 456) / 2) as i16 as i32;
    wr(d, p + (12036), -acc);
    acc = (-acc / 2 + rd(d, p + (12036) - 456)) as i16 as i32;
    wr(d, p + (13710), acc);
    acc = (rd(d, p + (13710) - 878) * 3 / 4 + rd(d, p + (11578) - 331) / 2) as i16 as i32;
    wr(d, p + (11578), -acc);
    acc = (-acc / 2 + rd(d, p + (11578) - 331) + rd(d, p + (6809) - 1) / 2) as i16 as i32;
    wr(d, p + (6809), acc);
    acc = (rd(d, p + (6809) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (15952), acc);
    acc = (rd(d, p + (11245) - 573) * 3 / 4 + rd(d, p + (8279) - 412) / 2) as i16 as i32;
    wr(d, p + (8279), -acc);
    acc = (-acc / 2 + rd(d, p + (8279) - 412)) as i16 as i32;
    wr(d, p + (10670), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (10670) - 681) * 3 / 4 + rd(d, p + (7865) - 381) / 2) as i16 as i32;
    wr(d, p + (7865), -acc);
    acc = (-acc / 2 + rd(d, p + (7865) - 381)) as i16 as i32;
    wr(d, p + (9987), acc);
    acc = (rd(d, p + (9987) - 871) * 3 / 4 + rd(d, p + (7482) - 434) / 2) as i16 as i32;
    wr(d, p + (7482), -acc);
    acc = (-acc / 2 + rd(d, p + (7482) - 434)) as i16 as i32;
    wr(d, p + (9114), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (9114) - 833) * 3 / 4 + rd(d, p + (7046) - 232) / 2) as i16 as i32;
    wr(d, p + (7046), -acc);
    acc = (-acc / 2 + rd(d, p + (7046) - 232) + rd(d, p + (6806) - 1) / 2) as i16 as i32;
    wr(d, p + (6806), acc);
    acc = (rd(d, p + (6806) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (11245), acc);
    (left, right)
}

fn midiverb2_effect_9(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 120) * 9 / 8 + rd(d, p + (14952) - 334) * 9 / 8 + rd(d, p + (14138) - 252) * 9 / 8 + rd(d, p + (13180) - 478) * 9 / 8 + rd(d, p + (9647) - 369) * 9 / 8 + rd(d, p + (8772) - 499) * 9 / 8 + rd(d, p + (7889) - 323) * 9 / 8) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (14952) - 130) * 9 / 8 + rd(d, p + (14138) - 459) * 9 / 8 + rd(d, p + (13180) - 277) * 9 / 8 + rd(d, p + (9647) - 363) * 9 / 8 + rd(d, p + (8772) - 208) * 9 / 8 + rd(d, p + (6916) - 491) * 9 / 8) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (3037) - 1) / 2) as i16 as i32;
    wr(d, p + (3037), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (3037) - 0) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 968) * 5 / 8 + rd(d, p + (12432) - 678) / 2) as i16 as i32;
    wr(d, p + (12432), -acc);
    acc = (-acc / 2 + rd(d, p + (12432) - 678)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 812) / 2 + rd(d, p + (11752) - 512) / 2) as i16 as i32;
    wr(d, p + (11752), -acc);
    acc = (-acc / 2 + rd(d, p + (11752) - 512)) as i16 as i32;
    wr(d, p + (14138), acc);
    acc = (rd(d, p + (14138) - 956) * 5 / 8 + rd(d, p + (11238) - 756) / 2) as i16 as i32;
    wr(d, p + (11238), -acc);
    acc = (-acc / 2 + rd(d, p + (11238) - 756)) as i16 as i32;
    wr(d, p + (13180), acc);
    acc = (rd(d, p + (13180) - 746) * 5 / 8 + rd(d, p + (10480) - 831) / 2) as i16 as i32;
    wr(d, p + (10480), -acc);
    acc = (-acc / 2 + rd(d, p + (10480) - 831) + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9647) - 873) * 5 / 8 + rd(d, p + (5904) - 512) / 2) as i16 as i32;
    wr(d, p + (5904), -acc);
    acc = (-acc / 2 + rd(d, p + (5904) - 512)) as i16 as i32;
    wr(d, p + (8772), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8772) - 881) / 2 + rd(d, p + (5390) - 781) / 2) as i16 as i32;
    wr(d, p + (5390), -acc);
    acc = (-acc / 2 + rd(d, p + (5390) - 781)) as i16 as i32;
    wr(d, p + (7889), acc);
    acc = (rd(d, p + (7889) - 971) * 5 / 8 + rd(d, p + (4607) - 634) / 2) as i16 as i32;
    wr(d, p + (4607), -acc);
    acc = (-acc / 2 + rd(d, p + (4607) - 634)) as i16 as i32;
    wr(d, p + (6916), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (6916) - 1010) * 5 / 8 + rd(d, p + (3971) - 932) / 2) as i16 as i32;
    wr(d, p + (3971), -acc);
    acc = (-acc / 2 + rd(d, p + (3971) - 932) + tmp_b) as i16 as i32;
    wr(d, p + (9647), acc);
    (left, right)
}

fn midiverb2_effect_10(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 120) * 9 / 8 + rd(d, p + (14952) - 334) * 9 / 8 + rd(d, p + (14138) - 252) * 9 / 8 + rd(d, p + (13180) - 478) * 9 / 8 + rd(d, p + (9647) - 369) * 9 / 8 + rd(d, p + (8772) - 499) * 9 / 8 + rd(d, p + (7889) - 323) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (14952) - 130) * 9 / 8 + rd(d, p + (14138) - 459) * 9 / 8 + rd(d, p + (13180) - 277) * 9 / 8 + rd(d, p + (9647) - 363) * 9 / 8 + rd(d, p + (8772) - 208) * 9 / 8 + rd(d, p + (6916) - 491) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 968) * 5 / 8 + rd(d, p + (12432) - 678) / 2) as i16 as i32;
    wr(d, p + (12432), -acc);
    acc = (-acc / 2 + rd(d, p + (12432) - 678)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 812) * 5 / 8 + rd(d, p + (11752) - 512) / 2) as i16 as i32;
    wr(d, p + (11752), -acc);
    acc = (-acc / 2 + rd(d, p + (11752) - 512)) as i16 as i32;
    wr(d, p + (14138), acc);
    acc = (rd(d, p + (14138) - 956) * 5 / 8 + rd(d, p + (11238) - 756) / 2) as i16 as i32;
    wr(d, p + (11238), -acc);
    acc = (-acc / 2 + rd(d, p + (11238) - 756)) as i16 as i32;
    wr(d, p + (13180), acc);
    acc = (rd(d, p + (13180) - 746) * 5 / 8 + rd(d, p + (10480) - 831) / 2) as i16 as i32;
    wr(d, p + (10480), -acc);
    acc = (-acc / 2 + rd(d, p + (10480) - 831) + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9647) - 873) * 5 / 8 + rd(d, p + (5904) - 512) / 2) as i16 as i32;
    wr(d, p + (5904), -acc);
    acc = (-acc / 2 + rd(d, p + (5904) - 512)) as i16 as i32;
    wr(d, p + (8772), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8772) - 881) * 5 / 8 + rd(d, p + (5390) - 781) / 2) as i16 as i32;
    wr(d, p + (5390), -acc);
    acc = (-acc / 2 + rd(d, p + (5390) - 781)) as i16 as i32;
    wr(d, p + (7889), acc);
    acc = (rd(d, p + (7889) - 971) * 5 / 8 + rd(d, p + (4607) - 634) / 2) as i16 as i32;
    wr(d, p + (4607), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (4607) - 634)) as i16 as i32;
    wr(d, p + (6916), acc);
    acc = (rd(d, p + (6916) - 1010) * 5 / 8 + rd(d, p + (3971) - 932) / 2) as i16 as i32;
    wr(d, p + (3971), -acc);
    acc = (-acc / 2 + rd(d, p + (3971) - 932) + tmp_b) as i16 as i32;
    wr(d, p + (9647), acc);
    (left, right)
}

fn midiverb2_effect_11(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 120) * 9 / 8 + rd(d, p + (14952) - 334) * 9 / 8 + rd(d, p + (14138) - 252) * 9 / 8 + rd(d, p + (13180) - 478) * 9 / 8 + rd(d, p + (9647) - 369) * 9 / 8 + rd(d, p + (8772) - 499) * 9 / 8 + rd(d, p + (7889) - 323) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (14952) - 130) * 9 / 8 + rd(d, p + (14138) - 459) * 9 / 8 + rd(d, p + (13180) - 277) * 9 / 8 + rd(d, p + (9647) - 363) * 9 / 8 + rd(d, p + (8772) - 208) * 9 / 8 + rd(d, p + (6916) - 491) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 968) * 3 / 4 + rd(d, p + (12432) - 678) / 2) as i16 as i32;
    wr(d, p + (12432), -acc);
    acc = (-acc / 2 + rd(d, p + (12432) - 678)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 812) * 3 / 4 + rd(d, p + (11752) - 512) / 2) as i16 as i32;
    wr(d, p + (11752), -acc);
    acc = (-acc / 2 + rd(d, p + (11752) - 512)) as i16 as i32;
    wr(d, p + (14138), acc);
    acc = (rd(d, p + (14138) - 956) * 5 / 8 + rd(d, p + (11238) - 756) / 2) as i16 as i32;
    wr(d, p + (11238), -acc);
    acc = (-acc / 2 + rd(d, p + (11238) - 756)) as i16 as i32;
    wr(d, p + (13180), acc);
    acc = (rd(d, p + (13180) - 746) * 5 / 8 + rd(d, p + (10480) - 831) / 2) as i16 as i32;
    wr(d, p + (10480), -acc);
    acc = (-acc / 2 + rd(d, p + (10480) - 831) + rd(d, p + (3034) - 1) / 2) as i16 as i32;
    wr(d, p + (3034), acc);
    acc = (rd(d, p + (3034) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9647) - 873) * 3 / 4 + rd(d, p + (5904) - 512) / 2) as i16 as i32;
    wr(d, p + (5904), -acc);
    acc = (-acc / 2 + rd(d, p + (5904) - 512)) as i16 as i32;
    wr(d, p + (8772), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8772) - 881) * 3 / 4 + rd(d, p + (5390) - 781) / 2) as i16 as i32;
    wr(d, p + (5390), -acc);
    acc = (-acc / 2 + rd(d, p + (5390) - 781)) as i16 as i32;
    wr(d, p + (7889), acc);
    acc = (rd(d, p + (7889) - 971) * 5 / 8 + rd(d, p + (4607) - 634) / 2) as i16 as i32;
    wr(d, p + (4607), -acc);
    acc = (-acc / 2 + rd(d, p + (4607) - 634)) as i16 as i32;
    wr(d, p + (6916), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (6916) - 1010) * 5 / 8 + rd(d, p + (3971) - 932) / 2) as i16 as i32;
    wr(d, p + (3971), -acc);
    acc = (-acc / 2 + rd(d, p + (3971) - 932) + rd(d, p + (3031) - 1) / 2) as i16 as i32;
    wr(d, p + (3031), acc);
    acc = (rd(d, p + (3031) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (9647), acc);
    (left, right)
}

fn midiverb2_effect_12(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15417) - 37) * 3 / 4 + rd(d, p + (14594) - 336) * 3 / 4 + rd(d, p + (13980) - 454) * 3 / 4 + rd(d, p + (13222) - 478) * 3 / 4 + rd(d, p + (9824) - 569) * 3 / 4 + rd(d, p + (8887) - 499) * 3 / 4 + rd(d, p + (7904) - 723) * 3 / 4 + rd(d, p + (7031) - 391) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15417) - 370) * 3 / 4 + rd(d, p + (14594) - 230) * 3 / 4 + rd(d, p + (13980) - 457) * 3 / 4 + rd(d, p + (13222) - 477) * 3 / 4 + rd(d, p + (9824) - 569) * 3 / 4 + rd(d, p + (8887) - 208) * 3 / 4 + rd(d, p + (7904) - 223) * 3 / 4 + rd(d, p + (7031) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (3852) - 1) / 2 + rd(d, p + (0) - 451) / 2) as i16 as i32;
    wr(d, p + (3852), acc);
    acc = (rd(d, p + (15931) - 7) * 3 / 4 + rd(d, p + (0) - 451) / 2) as i16 as i32;
    wr(d, p + (15931), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15922) - 15) * 3 / 4 + rd(d, p + (15931) - 7)) as i16 as i32;
    wr(d, p + (15922), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15905) - 34) * 3 / 4 + rd(d, p + (15922) - 15)) as i16 as i32;
    wr(d, p + (15905), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15869) - 75) * 3 / 4 + rd(d, p + (15905) - 34)) as i16 as i32;
    wr(d, p + (15869), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15792) - 135) * 3 / 4 + rd(d, p + (15869) - 75)) as i16 as i32;
    wr(d, p + (15792), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15655) - 236) * 3 / 4 + rd(d, p + (15792) - 135)) as i16 as i32;
    wr(d, p + (15655), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (15655) - 236) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15417) - 821) / 2 + rd(d, p + (15417) - 822) / 2 + rd(d, p + (12242) - 789) / 2) as i16 as i32;
    wr(d, p + (12242), -acc);
    acc = (-acc / 2 + rd(d, p + (12242) - 789)) as i16 as i32;
    wr(d, p + (14594), acc);
    acc = (rd(d, p + (14594) - 612) / 2 + rd(d, p + (14594) - 613) / 2 + rd(d, p + (11451) - 412) / 2) as i16 as i32;
    wr(d, p + (11451), -acc);
    acc = (-acc / 2 + rd(d, p + (11451) - 412)) as i16 as i32;
    wr(d, p + (13980), acc);
    acc = (rd(d, p + (13980) - 756) * -3 / 8 + rd(d, p + (13980) - 757) * -3 / 8 + rd(d, p + (11037) - 678) / 2) as i16 as i32;
    wr(d, p + (11037), -acc);
    acc = (-acc / 2 + rd(d, p + (11037) - 678)) as i16 as i32;
    wr(d, p + (13222), acc);
    acc = (rd(d, p + (13222) - 978) / 2 + rd(d, p + (10357) - 531) / 2) as i16 as i32;
    wr(d, p + (10357), -acc);
    acc = (-acc / 2 + rd(d, p + (10357) - 531)) as i16 as i32;
    wr(d, p + (9824), acc);
    acc = (rd(d, p + (9824) - 935) * 3 / 4 + rd(d, p + (5819) - 612) / 2) as i16 as i32;
    wr(d, p + (5819), -acc);
    acc = (-acc / 2 + rd(d, p + (5819) - 612)) as i16 as i32;
    wr(d, p + (8887), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8887) - 981) * -3 / 8 + rd(d, p + (8887) - 982) * -3 / 8 + rd(d, p + (5205) - 381) / 2) as i16 as i32;
    wr(d, p + (5205), -acc);
    acc = (-acc / 2 + rd(d, p + (5205) - 381)) as i16 as i32;
    wr(d, p + (7904), acc);
    acc = (rd(d, p + (7904) - 871) * 3 / 4 + rd(d, p + (4822) - 534) / 2) as i16 as i32;
    wr(d, p + (4822), -acc);
    acc = (-acc / 2 + rd(d, p + (4822) - 534)) as i16 as i32;
    wr(d, p + (7031), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (7031) - 1210) * 3 / 4 + rd(d, p + (4286) - 432) / 2) as i16 as i32;
    wr(d, p + (4286), -acc);
    acc = (acc / 4 + -rd(d, p + (4286) - 432) / 2 + rd(d, p + (3846) - 1) / 2) as i16 as i32;
    wr(d, p + (3846), acc);
    acc = (rd(d, p + (3846) - 0) + tmp_b) as i16 as i32;
    wr(d, p + (15417), acc);
    (left, right)
}

fn midiverb2_effect_13(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15417) - 37) * 3 / 4 + rd(d, p + (14794) - 236) * 3 / 4 + rd(d, p + (14380) - 454) * 3 / 4 + rd(d, p + (13922) - 478) * 3 / 4 + rd(d, p + (10081) - 569) * 3 / 4 + rd(d, p + (9144) - 499) * 3 / 4 + rd(d, p + (8161) - 723) * 3 / 4 + rd(d, p + (7288) - 391) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15417) - 37) * 3 / 4 + rd(d, p + (14794) - 230) * 3 / 4 + rd(d, p + (14380) - 457) * 3 / 4 + rd(d, p + (13922) - 477) * 3 / 4 + rd(d, p + (10081) - 569) * 3 / 4 + rd(d, p + (9144) - 508) * 3 / 4 + rd(d, p + (8161) - 223) * 3 / 4 + rd(d, p + (7288) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (3509) - 1) / 2 + rd(d, p + (0) - 451) / 2) as i16 as i32;
    wr(d, p + (3509), acc);
    acc = (rd(d, p + (15931) - 7) * 3 / 4 + rd(d, p + (0) - 451) / 2) as i16 as i32;
    wr(d, p + (15931), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15922) - 15) * 3 / 4 + rd(d, p + (15931) - 7)) as i16 as i32;
    wr(d, p + (15922), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15905) - 34) * 3 / 4 + rd(d, p + (15922) - 15)) as i16 as i32;
    wr(d, p + (15905), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15869) - 75) * 3 / 4 + rd(d, p + (15905) - 34)) as i16 as i32;
    wr(d, p + (15869), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15792) - 135) * 3 / 4 + rd(d, p + (15869) - 75)) as i16 as i32;
    wr(d, p + (15792), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (15655) - 236) * 3 / 4 + rd(d, p + (15792) - 135)) as i16 as i32;
    wr(d, p + (15655), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (15655) - 236) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15417) - 621) / 2 + rd(d, p + (15417) - 622) / 2 + rd(d, p + (13342) - 789) / 2) as i16 as i32;
    wr(d, p + (13342), -acc);
    acc = (-acc / 2 + rd(d, p + (13342) - 789)) as i16 as i32;
    wr(d, p + (14794), acc);
    acc = (rd(d, p + (14794) - 412) / 2 + rd(d, p + (14794) - 413) / 2 + rd(d, p + (12551) - 912) / 2) as i16 as i32;
    wr(d, p + (12551), -acc);
    acc = (-acc / 2 + rd(d, p + (12551) - 912)) as i16 as i32;
    wr(d, p + (14380), acc);
    acc = (rd(d, p + (14380) - 456) * -3 / 8 + rd(d, p + (14380) - 457) * -3 / 8 + rd(d, p + (11637) - 1021) / 2) as i16 as i32;
    wr(d, p + (11637), -acc);
    acc = (-acc / 2 + rd(d, p + (11637) - 1021)) as i16 as i32;
    wr(d, p + (13922), acc);
    acc = (rd(d, p + (13922) - 578) / 2 + rd(d, p + (10614) - 531) / 2) as i16 as i32;
    wr(d, p + (10614), -acc);
    acc = (-acc / 2 + rd(d, p + (10614) - 531)) as i16 as i32;
    wr(d, p + (10081), acc);
    acc = (rd(d, p + (10081) - 935) * 3 / 4 + rd(d, p + (6076) - 712) / 2) as i16 as i32;
    wr(d, p + (6076), -acc);
    acc = (-acc / 2 + rd(d, p + (6076) - 712)) as i16 as i32;
    wr(d, p + (9144), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (9144) - 981) * -3 / 8 + rd(d, p + (9144) - 982) * -3 / 8 + rd(d, p + (5362) - 381) / 2) as i16 as i32;
    wr(d, p + (5362), -acc);
    acc = (-acc / 2 + rd(d, p + (5362) - 381)) as i16 as i32;
    wr(d, p + (8161), acc);
    acc = (rd(d, p + (8161) - 871) * 3 / 4 + rd(d, p + (4979) - 634) / 2) as i16 as i32;
    wr(d, p + (4979), -acc);
    acc = (-acc / 2 + rd(d, p + (4979) - 634)) as i16 as i32;
    wr(d, p + (7288), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (7288) - 1210) * 3 / 4 + rd(d, p + (4343) - 832) / 2) as i16 as i32;
    wr(d, p + (4343), -acc);
    acc = (acc / 4 + -rd(d, p + (4343) - 832) / 2 + rd(d, p + (3503) - 1) / 2) as i16 as i32;
    wr(d, p + (3503), acc);
    acc = (rd(d, p + (3503) - 0) + tmp_b) as i16 as i32;
    wr(d, p + (15417), acc);
    (left, right)
}

fn midiverb2_effect_14(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 120) * 9 / 8 + rd(d, p + (15252) - 334) * 9 / 8 + rd(d, p + (14438) - 252) * 9 / 8 + rd(d, p + (13480) - 478) * 9 / 8 + rd(d, p + (9683) - 369) * 9 / 8 + rd(d, p + (9008) - 499) * 9 / 8 + rd(d, p + (8125) - 723) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (15252) - 130) * 9 / 8 + rd(d, p + (14438) - 459) * 9 / 8 + rd(d, p + (13480) - 277) * 9 / 8 + rd(d, p + (9683) - 363) * 9 / 8 + rd(d, p + (9008) - 208) * 9 / 8 + rd(d, p + (7152) - 891) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (3073) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (3073), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (3073) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 668) * 3 / 4 + rd(d, p + (12468) - 678) / 2) as i16 as i32;
    wr(d, p + (12468), -acc);
    acc = (-acc / 2 + rd(d, p + (12468) - 678)) as i16 as i32;
    wr(d, p + (15252), acc);
    acc = (rd(d, p + (15252) - 812) * 3 / 4 + rd(d, p + (11788) - 512) / 2) as i16 as i32;
    wr(d, p + (11788), -acc);
    acc = (-acc / 2 + rd(d, p + (11788) - 512)) as i16 as i32;
    wr(d, p + (14438), acc);
    acc = (rd(d, p + (14438) - 956) * 3 / 4 + rd(d, p + (11274) - 756) / 2) as i16 as i32;
    wr(d, p + (11274), -acc);
    acc = (-acc / 2 + rd(d, p + (11274) - 756)) as i16 as i32;
    wr(d, p + (13480), acc);
    acc = (rd(d, p + (13480) - 1010) * 3 / 4 + rd(d, p + (10516) - 831) / 2) as i16 as i32;
    wr(d, p + (10516), -acc);
    acc = (-acc / 2 + rd(d, p + (10516) - 831) + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9683) - 673) * 3 / 4 + rd(d, p + (5940) - 512) / 2) as i16 as i32;
    wr(d, p + (5940), -acc);
    acc = (-acc / 2 + rd(d, p + (5940) - 512)) as i16 as i32;
    wr(d, p + (9008), acc);
    acc = (rd(d, p + (9008) - 881) * 3 / 4 + rd(d, p + (5426) - 781) / 2) as i16 as i32;
    wr(d, p + (5426), -acc);
    acc = (-acc / 2 + rd(d, p + (5426) - 781)) as i16 as i32;
    wr(d, p + (8125), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8125) - 971) * 3 / 4 + rd(d, p + (4643) - 634) / 2) as i16 as i32;
    wr(d, p + (4643), -acc);
    acc = (-acc / 2 + rd(d, p + (4643) - 634)) as i16 as i32;
    wr(d, p + (7152), acc);
    acc = (rd(d, p + (7152) - 1210) * 3 / 4 + rd(d, p + (4007) - 932) / 2) as i16 as i32;
    wr(d, p + (4007), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (4007) - 932) + tmp_b) as i16 as i32;
    wr(d, p + (9683), acc);
    (left, right)
}

fn midiverb2_effect_15(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 520) * 9 / 8 + rd(d, p + (14952) - 334) * 9 / 8 + rd(d, p + (14138) - 252) * 9 / 8 + rd(d, p + (13180) - 478) * 9 / 8 + rd(d, p + (9183) - 369) * 9 / 8 + rd(d, p + (8408) - 499) * 9 / 8 + rd(d, p + (7525) - 723) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (14952) - 130) * 9 / 8 + rd(d, p + (14138) - 459) * 9 / 8 + rd(d, p + (13180) - 277) * 9 / 8 + rd(d, p + (9183) - 363) * 9 / 8 + rd(d, p + (8408) - 208) * 9 / 8 + rd(d, p + (6552) - 891) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (2473) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (2473), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 968) * 3 / 4 + rd(d, p + (12068) - 678) / 2) as i16 as i32;
    wr(d, p + (12068), -acc);
    acc = (-acc / 2 + rd(d, p + (12068) - 678)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 812) * 3 / 4 + rd(d, p + (11388) - 612) / 2) as i16 as i32;
    wr(d, p + (11388), -acc);
    acc = (-acc / 2 + rd(d, p + (11388) - 612)) as i16 as i32;
    wr(d, p + (14138), acc);
    acc = (rd(d, p + (14138) - 956) * 3 / 4 + rd(d, p + (10774) - 756) / 2) as i16 as i32;
    wr(d, p + (10774), -acc);
    acc = (-acc / 2 + rd(d, p + (10774) - 756)) as i16 as i32;
    wr(d, p + (13180), acc);
    acc = (rd(d, p + (13180) - 1110) * 3 / 4 + rd(d, p + (10016) - 831) / 2) as i16 as i32;
    wr(d, p + (10016), -acc);
    acc = (-acc / 2 + rd(d, p + (10016) - 831) + rd(d, p + (2470) - 1) / 2) as i16 as i32;
    wr(d, p + (2470), acc);
    acc = (rd(d, p + (2470) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9183) - 773) * 3 / 4 + rd(d, p + (5340) - 512) / 2) as i16 as i32;
    wr(d, p + (5340), -acc);
    acc = (-acc / 2 + rd(d, p + (5340) - 512)) as i16 as i32;
    wr(d, p + (8408), acc);
    acc = (rd(d, p + (8408) - 881) * 3 / 4 + rd(d, p + (4826) - 781) / 2) as i16 as i32;
    wr(d, p + (4826), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (4826) - 781)) as i16 as i32;
    wr(d, p + (7525), acc);
    acc = (rd(d, p + (7525) - 971) * 3 / 4 + rd(d, p + (4043) - 634) / 2) as i16 as i32;
    wr(d, p + (4043), -acc);
    acc = (-acc / 2 + rd(d, p + (4043) - 634)) as i16 as i32;
    wr(d, p + (6552), acc);
    acc = (rd(d, p + (6552) - 1210) * 3 / 4 + rd(d, p + (3407) - 932) / 2) as i16 as i32;
    wr(d, p + (3407), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (3407) - 932) + rd(d, p + (2467) - 1) / 2) as i16 as i32;
    wr(d, p + (2467), acc);
    acc = (rd(d, p + (2467) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (9183), acc);
    (left, right)
}

fn midiverb2_effect_16(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 120) * 9 / 8 + rd(d, p + (15252) - 334) * 9 / 8 + rd(d, p + (14438) - 252) * 9 / 8 + rd(d, p + (13480) - 478) * 9 / 8 + rd(d, p + (9683) - 369) * 9 / 8 + rd(d, p + (9008) - 499) * 9 / 8 + rd(d, p + (8125) - 723) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (15252) - 130) * 9 / 8 + rd(d, p + (14438) - 459) * 9 / 8 + rd(d, p + (13480) - 277) * 9 / 8 + rd(d, p + (9683) - 363) * 9 / 8 + rd(d, p + (9008) - 208) * 9 / 8 + rd(d, p + (7152) - 891) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (3073) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (3073), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 668) * 3 / 4 + rd(d, p + (12468) - 678) / 2) as i16 as i32;
    wr(d, p + (12468), -acc);
    acc = (-acc / 2 + rd(d, p + (12468) - 678)) as i16 as i32;
    wr(d, p + (15252), acc);
    acc = (rd(d, p + (15252) - 812) * 3 / 4 + rd(d, p + (11788) - 512) / 2) as i16 as i32;
    wr(d, p + (11788), -acc);
    acc = (-acc / 2 + rd(d, p + (11788) - 512)) as i16 as i32;
    wr(d, p + (14438), acc);
    acc = (rd(d, p + (14438) - 956) * 3 / 4 + rd(d, p + (11274) - 756) / 2) as i16 as i32;
    wr(d, p + (11274), -acc);
    acc = (-acc / 2 + rd(d, p + (11274) - 756)) as i16 as i32;
    wr(d, p + (13480), acc);
    acc = (rd(d, p + (13480) - 1010) * 3 / 4 + rd(d, p + (10516) - 831) / 2) as i16 as i32;
    wr(d, p + (10516), -acc);
    acc = (-acc / 2 + rd(d, p + (10516) - 831) + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (9683) - 673) * 3 / 4 + rd(d, p + (5940) - 512) / 2) as i16 as i32;
    wr(d, p + (5940), -acc);
    acc = (-acc / 2 + rd(d, p + (5940) - 512)) as i16 as i32;
    wr(d, p + (9008), acc);
    acc = (rd(d, p + (9008) - 881) * 3 / 4 + rd(d, p + (5426) - 781) / 2) as i16 as i32;
    wr(d, p + (5426), -acc);
    acc = (-acc / 2 + rd(d, p + (5426) - 781)) as i16 as i32;
    wr(d, p + (8125), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (8125) - 971) * 3 / 4 + rd(d, p + (4643) - 634) / 2) as i16 as i32;
    wr(d, p + (4643), -acc);
    acc = (-acc / 2 + rd(d, p + (4643) - 634)) as i16 as i32;
    wr(d, p + (7152), acc);
    acc = (rd(d, p + (7152) - 1210) * 3 / 4 + rd(d, p + (4007) - 932) / 2) as i16 as i32;
    wr(d, p + (4007), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (4007) - 932) + tmp_b) as i16 as i32;
    wr(d, p + (9683), acc);
    (left, right)
}

fn midiverb2_effect_17(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13337) - 211) * 3 / 4 + rd(d, p + (10417) - 1391) * 3 / 4 + rd(d, p + (6832) - 1403) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13337) - 211) * 3 / 4 + rd(d, p + (10417) - 690) * 3 / 4 + rd(d, p + (6832) - 471) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 457) / 2 + rd(d, p + (3759) - 1) / 2) as i16 as i32;
    wr(d, p + (3759), acc);
    acc = (rd(d, p + (0) - 457) / 2 + rd(d, p + (15925) - 321) / 2) as i16 as i32;
    wr(d, p + (15925), -acc);
    acc = (-acc / 2 + rd(d, p + (15925) - 321) + rd(d, p + (15602) - 431) / 2) as i16 as i32;
    wr(d, p + (15602), -acc);
    acc = (-acc / 2 + rd(d, p + (15602) - 431) + rd(d, p + (15169) - 668) / 2) as i16 as i32;
    wr(d, p + (15169), -acc);
    acc = (-acc / 2 + rd(d, p + (15169) - 668) + rd(d, p + (14499) - 808) / 2) as i16 as i32;
    wr(d, p + (14499), -acc);
    acc = (-acc / 2 + rd(d, p + (14499) - 808) + rd(d, p + (13689) - 204) / 2) as i16 as i32;
    wr(d, p + (13689), -acc);
    acc = (-acc / 2 + rd(d, p + (13689) - 204) + rd(d, p + (13483) - 144) / 2) as i16 as i32;
    wr(d, p + (13483), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (13483) - 144) * 9 / 4) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13337) - 1724) * 3 / 4 + rd(d, p + (11611) - 1192) / 2) as i16 as i32;
    wr(d, p + (11611), -acc);
    acc = (-acc / 2 + rd(d, p + (11611) - 1192)) as i16 as i32;
    wr(d, p + (10417), acc);
    acc = (rd(d, p + (10417) - 2510) * 5 / 8 + rd(d, p + (7905) - 1071) / 2) as i16 as i32;
    wr(d, p + (7905), -acc);
    acc = (-acc / 2 + rd(d, p + (7905) - 1071)) as i16 as i32;
    wr(d, p + (6832), acc);
    acc = (rd(d, p + (6832) - 1857) * 5 / 8 + rd(d, p + (4973) - 1212) / 2) as i16 as i32;
    wr(d, p + (4973), -acc);
    acc = (-acc / 2 + rd(d, p + (4973) - 1212) + rd(d, p + (3756) - 1) / 2) as i16 as i32;
    wr(d, p + (3756), acc);
    acc = (rd(d, p + (3756) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13337), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_18(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13793) - 211) * 3 / 4 + rd(d, p + (10673) - 1391) * 3 / 4 + rd(d, p + (6788) - 1403) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13793) - 1101) * 3 / 4 + rd(d, p + (10673) - 690) * 3 / 4 + rd(d, p + (6788) - 471) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (3415) - 1) / 2) as i16 as i32;
    wr(d, p + (3415), acc);
    acc = (rd(d, p + (3415) - 0) / 2 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 668) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 668) + rd(d, p + (14955) - 808) / 2) as i16 as i32;
    wr(d, p + (14955), -acc);
    acc = (-acc / 2 + rd(d, p + (14955) - 808) + rd(d, p + (14145) - 204) / 2) as i16 as i32;
    wr(d, p + (14145), -acc);
    acc = (-acc / 2 + rd(d, p + (14145) - 204) + rd(d, p + (13939) - 144) / 2) as i16 as i32;
    wr(d, p + (13939), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (13939) - 144) * 9 / 4) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13793) - 1724) * 3 / 4 + rd(d, p + (12067) - 1392) / 2) as i16 as i32;
    wr(d, p + (12067), -acc);
    acc = (-acc / 2 + rd(d, p + (12067) - 1392)) as i16 as i32;
    wr(d, p + (10673), acc);
    acc = (rd(d, p + (10673) - 2510) * 5 / 8 + rd(d, p + (8161) - 1371) / 2) as i16 as i32;
    wr(d, p + (8161), -acc);
    acc = (-acc / 2 + rd(d, p + (8161) - 1371)) as i16 as i32;
    wr(d, p + (6788), acc);
    acc = (rd(d, p + (6788) - 1857) * 5 / 8 + rd(d, p + (4929) - 1512) / 2) as i16 as i32;
    wr(d, p + (4929), -acc);
    acc = (-acc / 2 + rd(d, p + (4929) - 1512) + rd(d, p + (3412) - 1) / 2) as i16 as i32;
    wr(d, p + (3412), acc);
    acc = (rd(d, p + (3412) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13793), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_19(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13254) - 64) * 3 / 4 + rd(d, p + (10744) - 696) * 3 / 4 + rd(d, p + (5956) - 539) * 3 / 4 + rd(d, p + (0) - 178) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (13254) - 503) * 3 / 4 + rd(d, p + (10744) - 167) * 3 / 4 + rd(d, p + (5956) - 151) * 3 / 4 + rd(d, p + (0) - 511) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (14382) - 58) * 5 / 8 + rd(d, p + (0) - 2000) / 2) as i16 as i32;
    wr(d, p + (14382), -acc);
    acc = (rd(d, p + (14322) - 89) * 5 / 8 + rd(d, p + (14382) - 0) * 5 / 8 + rd(d, p + (14382) - 58)) as i16 as i32;
    wr(d, p + (14322), -acc);
    acc = (rd(d, p + (14231) - 183) * 5 / 8 + rd(d, p + (14322) - 0) * 5 / 8 + rd(d, p + (14322) - 89)) as i16 as i32;
    wr(d, p + (14231), -acc);
    acc = (rd(d, p + (14046) - 321) * 5 / 8 + rd(d, p + (14231) - 0) * 5 / 8 + rd(d, p + (14231) - 183)) as i16 as i32;
    wr(d, p + (14046), -acc);
    acc = (rd(d, p + (13723) - 467) * 5 / 8 + rd(d, p + (14046) - 0) * 5 / 8 + rd(d, p + (14046) - 321)) as i16 as i32;
    wr(d, p + (13723), -acc);
    acc = (acc * -3 / 16 + rd(d, p + (13723) - 0) * 3 / 4 + rd(d, p + (13723) - 467) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13254) - 1385) * 3 / 4 + rd(d, p + (11867) - 1121) / 2) as i16 as i32;
    wr(d, p + (11867), -acc);
    acc = (-acc / 2 + rd(d, p + (11867) - 1121)) as i16 as i32;
    wr(d, p + (10744), acc);
    acc = (rd(d, p + (10744) - 1641) * 3 / 4 + rd(d, p + (9101) - 1020) / 2) as i16 as i32;
    wr(d, p + (9101), -acc);
    acc = (-acc / 2 + rd(d, p + (9101) - 1020)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3434) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3434), acc);
    acc = (rd(d, p + (3434) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (13254), acc);
    acc = (rd(d, p + (8079) - 1211) * 3 / 4 + rd(d, p + (6866) - 908) / 2) as i16 as i32;
    wr(d, p + (6866), -acc);
    acc = (-acc / 2 + rd(d, p + (6866) - 908)) as i16 as i32;
    wr(d, p + (5956), acc);
    acc = (rd(d, p + (5956) - 1417) * 3 / 4 + rd(d, p + (4537) - 1098) / 2) as i16 as i32;
    wr(d, p + (4537), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (4537) - 1098)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (3431) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (3431), acc);
    acc = (rd(d, p + (3431) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (8079), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_20(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_1 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15253) - 64) * 3 / 4 + rd(d, p + (13283) - 696) * 3 / 4 + rd(d, p + (8586) - 539) * 3 / 4 + rd(d, p + (16230) - 24) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (15253) - 503) * 3 / 4 + rd(d, p + (13283) - 167) * 3 / 4 + rd(d, p + (8586) - 151) * 3 / 4 + rd(d, p + (16045) - 172) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 58) * 5 / 8 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (rd(d, p + (16321) - 89) * 5 / 8 + rd(d, p + (16381) - 0) * 5 / 8 + rd(d, p + (16381) - 58)) as i16 as i32;
    wr(d, p + (16321), -acc);
    acc = (rd(d, p + (16230) - 183) * 5 / 8 + rd(d, p + (16321) - 0) * 5 / 8 + rd(d, p + (16321) - 89)) as i16 as i32;
    wr(d, p + (16230), -acc);
    acc = (rd(d, p + (16045) - 321) * 5 / 8 + rd(d, p + (16230) - 0) * 5 / 8 + rd(d, p + (16230) - 183)) as i16 as i32;
    wr(d, p + (16045), -acc);
    acc = (rd(d, p + (15722) - 467) * 5 / 8 + rd(d, p + (16045) - 0) * 5 / 8 + rd(d, p + (16045) - 321)) as i16 as i32;
    wr(d, p + (15722), -acc);
    acc = (acc * -3 / 16 + rd(d, p + (15722) - 0) * 3 / 4 + rd(d, p + (15722) - 467) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15253) - 845) * 3 / 4 + rd(d, p + (14406) - 1121) / 2) as i16 as i32;
    wr(d, p + (14406), -acc);
    acc = (-acc / 2 + rd(d, p + (14406) - 1121)) as i16 as i32;
    wr(d, p + (13283), acc);
    acc = (rd(d, p + (13283) - 1441) * 3 / 4 + rd(d, p + (11840) - 1320) / 2) as i16 as i32;
    wr(d, p + (11840), -acc);
    acc = (-acc / 2 + rd(d, p + (11840) - 1320)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6264) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6264), acc);
    acc = (rd(d, p + (6264) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (15253), acc);
    acc = (rd(d, p + (10518) - 912) * 3 / 4 + rd(d, p + (9604) - 1016) / 2) as i16 as i32;
    wr(d, p + (9604), -acc);
    acc = (-acc / 2 + rd(d, p + (9604) - 1016)) as i16 as i32;
    wr(d, p + (8586), acc);
    acc = (rd(d, p + (8586) - 1017) * 3 / 4 + rd(d, p + (7567) - 1298) / 2) as i16 as i32;
    wr(d, p + (7567), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (7567) - 1298)) as i16 as i32;
    tmp_1 = (acc) as i16 as i32;
    acc = (rd(d, p + (6261) - 1) / 2 + tmp_1 / 2) as i16 as i32;
    wr(d, p + (6261), acc);
    acc = (rd(d, p + (6261) - 1) + tmp_b) as i16 as i32;
    wr(d, p + (10518), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_21(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15822) - 120) * 3 / 4 + rd(d, p + (14952) - 634) * 3 / 4 + rd(d, p + (13905) - 552) * 3 / 4 + rd(d, p + (12769) - 478) * 3 / 4 + rd(d, p + (8225) - 369) * 3 / 4 + rd(d, p + (7182) - 208) * 3 / 4 + rd(d, p + (6199) - 723) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15822) - 129) * 3 / 4 + rd(d, p + (14952) - 130) * 3 / 4 + rd(d, p + (13905) - 359) * 3 / 4 + rd(d, p + (12769) - 277) * 3 / 4 + rd(d, p + (8225) - 363) * 3 / 4 + rd(d, p + (7182) - 499) * 3 / 4 + rd(d, p + (5096) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (365) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (365), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 276) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 276)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15822) - 868) * 3 / 4 + rd(d, p + (11800) - 878) / 2) as i16 as i32;
    wr(d, p + (11800), -acc);
    acc = (-acc / 2 + rd(d, p + (11800) - 878)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 1045) * 3 / 4 + rd(d, p + (10920) - 712) / 2) as i16 as i32;
    wr(d, p + (10920), -acc);
    acc = (-acc / 2 + rd(d, p + (10920) - 712)) as i16 as i32;
    wr(d, p + (13905), acc);
    acc = (rd(d, p + (13905) - 1134) / 2 + rd(d, p + (13905) - 1135) / 2 + rd(d, p + (10206) - 956) / 2) as i16 as i32;
    wr(d, p + (10206), -acc);
    acc = (-acc / 2 + rd(d, p + (10206) - 956)) as i16 as i32;
    wr(d, p + (12769), acc);
    acc = (rd(d, p + (12769) - 967) / 2 + rd(d, p + (12769) - 968) / 2 + rd(d, p + (9248) - 1021) / 2) as i16 as i32;
    wr(d, p + (9248), -acc);
    acc = (-acc / 2 + rd(d, p + (9248) - 1021) + tmp_b) as i16 as i32;
    wr(d, p + (15822), acc);
    acc = (rd(d, p + (8225) - 1041) * 3 / 4 + rd(d, p + (4084) - 812) / 2) as i16 as i32;
    wr(d, p + (4084), -acc);
    acc = (-acc / 2 + rd(d, p + (4084) - 812)) as i16 as i32;
    wr(d, p + (7182), acc);
    acc = (rd(d, p + (7182) - 981) * 3 / 4 + rd(d, p + (3270) - 981) / 2) as i16 as i32;
    wr(d, p + (3270), -acc);
    acc = (-acc / 2 + rd(d, p + (3270) - 981)) as i16 as i32;
    wr(d, p + (6199), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (6199) - 1101) / 2 + rd(d, p + (6199) - 1102) / 2 + rd(d, p + (2287) - 934) / 2) as i16 as i32;
    wr(d, p + (2287), -acc);
    acc = (-acc / 2 + rd(d, p + (2287) - 934)) as i16 as i32;
    wr(d, p + (5096), acc);
    acc = (rd(d, p + (5096) - 1010) / 2 + rd(d, p + (5096) - 1011) / 2 + rd(d, p + (1351) - 984) / 2) as i16 as i32;
    wr(d, p + (1351), -acc);
    acc = (-acc / 2 + rd(d, p + (1351) - 984) + tmp_b) as i16 as i32;
    wr(d, p + (8225), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_22(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15822) - 420) * 3 / 4 + rd(d, p + (14617) - 634) * 3 / 4 + rd(d, p + (13570) - 552) * 3 / 4 + rd(d, p + (12434) - 478) * 3 / 4 + rd(d, p + (9099) - 269) * 3 / 4 + rd(d, p + (8056) - 208) * 3 / 4 + rd(d, p + (7073) - 723) * 3 / 4) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15822) - 129) * 3 / 4 + rd(d, p + (14617) - 130) * 3 / 4 + rd(d, p + (13570) - 359) * 3 / 4 + rd(d, p + (12434) - 277) * 3 / 4 + rd(d, p + (9099) - 363) * 3 / 4 + rd(d, p + (8056) - 499) * 3 / 4 + rd(d, p + (5970) - 891) * 3 / 4) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (2619) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (2619), acc);
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 276) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 276)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15822) - 1203) * 3 / 4 + rd(d, p + (11465) - 578) / 2) as i16 as i32;
    wr(d, p + (11465), -acc);
    acc = (-acc / 2 + rd(d, p + (11465) - 578)) as i16 as i32;
    wr(d, p + (14617), acc);
    acc = (rd(d, p + (14617) - 1045) * 3 / 4 + rd(d, p + (10885) - 712) / 2) as i16 as i32;
    wr(d, p + (10885), -acc);
    acc = (-acc / 2 + rd(d, p + (10885) - 712)) as i16 as i32;
    wr(d, p + (13570), acc);
    acc = (rd(d, p + (13570) - 1134) / 2 + rd(d, p + (13570) - 1135) / 2 + rd(d, p + (10171) - 756) / 2) as i16 as i32;
    wr(d, p + (10171), -acc);
    acc = (-acc / 2 + rd(d, p + (10171) - 756)) as i16 as i32;
    wr(d, p + (12434), acc);
    acc = (rd(d, p + (12434) - 967) / 2 + rd(d, p + (12434) - 968) / 2 + rd(d, p + (9413) - 312) / 2) as i16 as i32;
    wr(d, p + (9413), -acc);
    acc = (-acc / 2 + rd(d, p + (9413) - 312) + tmp_b) as i16 as i32;
    wr(d, p + (15822), acc);
    acc = (rd(d, p + (9099) - 1041) * 3 / 4 + rd(d, p + (4958) - 612) / 2) as i16 as i32;
    wr(d, p + (4958), -acc);
    acc = (-acc / 2 + rd(d, p + (4958) - 612)) as i16 as i32;
    wr(d, p + (8056), acc);
    acc = (rd(d, p + (8056) - 981) * 3 / 4 + rd(d, p + (4344) - 781) / 2) as i16 as i32;
    wr(d, p + (4344), -acc);
    acc = (-acc / 2 + rd(d, p + (4344) - 781)) as i16 as i32;
    wr(d, p + (7073), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (7073) - 1101) / 2 + rd(d, p + (7073) - 1102) / 2 + rd(d, p + (3561) - 634) / 2) as i16 as i32;
    wr(d, p + (3561), -acc);
    acc = (-acc / 2 + rd(d, p + (3561) - 634)) as i16 as i32;
    wr(d, p + (5970), acc);
    acc = (rd(d, p + (5970) - 1010) / 2 + rd(d, p + (5970) - 1011) / 2 + rd(d, p + (2925) - 304) / 2) as i16 as i32;
    wr(d, p + (2925), -acc);
    acc = (-acc / 2 + rd(d, p + (2925) - 304) + tmp_b) as i16 as i32;
    wr(d, p + (9099), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_23(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13326) - 567) * 9 / 8 + rd(d, p + (9506) - 1891) * 9 / 8 + rd(d, p + (5621) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13326) - 1190) * 9 / 8 + rd(d, p + (9506) - 890) * 9 / 8 + rd(d, p + (5621) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (1148) - 1) / 2) as i16 as i32;
    wr(d, p + (1148), acc);
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 668) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 668) + rd(d, p + (14955) - 1235) / 2) as i16 as i32;
    wr(d, p + (14955), -acc);
    acc = (-acc / 2 + rd(d, p + (14955) - 1235) + rd(d, p + (13718) - 234) / 2) as i16 as i32;
    wr(d, p + (13718), -acc);
    acc = (-acc / 2 + rd(d, p + (13718) - 234) + rd(d, p + (13482) - 154) / 2) as i16 as i32;
    wr(d, p + (13482), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (13482) - 154) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13326) - 2224) / 2 + rd(d, p + (11100) - 1592) / 2) as i16 as i32;
    wr(d, p + (11100), -acc);
    acc = (-acc / 2 + rd(d, p + (11100) - 1592)) as i16 as i32;
    wr(d, p + (9506), acc);
    acc = (rd(d, p + (9506) - 2610) * 3 / 4 + rd(d, p + (6894) - 1271) / 2) as i16 as i32;
    wr(d, p + (6894), -acc);
    acc = (-acc / 2 + rd(d, p + (6894) - 1271)) as i16 as i32;
    wr(d, p + (5621), acc);
    acc = (rd(d, p + (5621) - 3357) / 2 + rd(d, p + (2262) - 1112) / 2) as i16 as i32;
    wr(d, p + (2262), -acc);
    acc = (-acc / 2 + rd(d, p + (2262) - 1112) + rd(d, p + (1145) - 1) / 2) as i16 as i32;
    wr(d, p + (1145), acc);
    acc = (rd(d, p + (1145) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13326), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_24(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12327) - 567) * 9 / 8 + rd(d, p + (9007) - 1891) * 9 / 8 + rd(d, p + (5159) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (12327) - 567) * 9 / 8 + rd(d, p + (9007) - 890) * 9 / 8 + rd(d, p + (5159) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1000) / 2 + rd(d, p + (1012) - 1) / 2) as i16 as i32;
    wr(d, p + (1012), acc);
    acc = (rd(d, p + (1012) - 1) / 2 + rd(d, p + (15382) - 321) / 2) as i16 as i32;
    wr(d, p + (15382), -acc);
    acc = (-acc / 2 + rd(d, p + (15382) - 321) + rd(d, p + (15059) - 431) / 2) as i16 as i32;
    wr(d, p + (15059), -acc);
    acc = (-acc / 2 + rd(d, p + (15059) - 431) + rd(d, p + (14626) - 668) / 2) as i16 as i32;
    wr(d, p + (14626), -acc);
    acc = (-acc / 2 + rd(d, p + (14626) - 668) + rd(d, p + (13956) - 1235) / 2) as i16 as i32;
    wr(d, p + (13956), -acc);
    acc = (-acc / 2 + rd(d, p + (13956) - 1235) + rd(d, p + (12719) - 234) / 2) as i16 as i32;
    wr(d, p + (12719), -acc);
    acc = (-acc / 2 + rd(d, p + (12719) - 234) + rd(d, p + (12483) - 154) / 2) as i16 as i32;
    wr(d, p + (12483), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (12483) - 154) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (12327) - 2224) + rd(d, p + (10101) - 1092) / 2) as i16 as i32;
    wr(d, p + (10101), -acc);
    acc = (-acc / 2 + rd(d, p + (10101) - 1092)) as i16 as i32;
    wr(d, p + (9007), acc);
    acc = (rd(d, p + (9007) - 2610) * 3 / 4 + rd(d, p + (6395) - 1234) / 2) as i16 as i32;
    wr(d, p + (6395), -acc);
    acc = (-acc / 2 + rd(d, p + (6395) - 1234)) as i16 as i32;
    wr(d, p + (5159), acc);
    acc = (rd(d, p + (5159) - 3357) / 2 + rd(d, p + (1800) - 786) / 2) as i16 as i32;
    wr(d, p + (1800), -acc);
    acc = (-acc / 2 + rd(d, p + (1800) - 786) + rd(d, p + (1009) - 1) / 2) as i16 as i32;
    wr(d, p + (1009), acc);
    acc = (rd(d, p + (1009) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (12327), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_25(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13326) - 567) * 9 / 8 + rd(d, p + (9506) - 1891) * 9 / 8 + rd(d, p + (5621) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13326) - 1190) * 9 / 8 + rd(d, p + (9506) - 890) * 9 / 8 + rd(d, p + (5621) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (1148) - 1) / 2) as i16 as i32;
    wr(d, p + (1148), acc);
    acc = (rd(d, p + (1148) - 1) / 2 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 668) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 668) + rd(d, p + (14955) - 1235) / 2) as i16 as i32;
    wr(d, p + (14955), -acc);
    acc = (-acc / 2 + rd(d, p + (14955) - 1235) + rd(d, p + (13718) - 234) / 2) as i16 as i32;
    wr(d, p + (13718), -acc);
    acc = (-acc / 2 + rd(d, p + (13718) - 234) + rd(d, p + (13482) - 154) / 2) as i16 as i32;
    wr(d, p + (13482), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (13482) - 154) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13326) - 2224) + rd(d, p + (11100) - 1592) / 2) as i16 as i32;
    wr(d, p + (11100), -acc);
    acc = (-acc / 2 + rd(d, p + (11100) - 1592)) as i16 as i32;
    wr(d, p + (9506), acc);
    acc = (rd(d, p + (9506) - 2610) * 3 / 4 + rd(d, p + (6894) - 1271) / 2) as i16 as i32;
    wr(d, p + (6894), -acc);
    acc = (-acc / 2 + rd(d, p + (6894) - 1271)) as i16 as i32;
    wr(d, p + (5621), acc);
    acc = (rd(d, p + (5621) - 3357) / 2 + rd(d, p + (2262) - 1112) / 2) as i16 as i32;
    wr(d, p + (2262), -acc);
    acc = (-acc / 2 + rd(d, p + (2262) - 1112) + rd(d, p + (1145) - 1) / 2) as i16 as i32;
    wr(d, p + (1145), acc);
    acc = (rd(d, p + (1145) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13326), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_26(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13526) - 567) * 9 / 8 + rd(d, p + (10206) - 1891) * 9 / 8 + rd(d, p + (6321) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13526) - 567) * 9 / 8 + rd(d, p + (10206) - 890) * 9 / 8 + rd(d, p + (6321) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (1848) - 1) / 2) as i16 as i32;
    wr(d, p + (1848), acc);
    acc = (rd(d, p + (0) - 1) / 2 + rd(d, p + (16381) - 321) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 668) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 668) + rd(d, p + (14955) - 1035) / 2) as i16 as i32;
    wr(d, p + (14955), -acc);
    acc = (-acc / 2 + rd(d, p + (14955) - 1035) + rd(d, p + (13918) - 234) / 2) as i16 as i32;
    wr(d, p + (13918), -acc);
    acc = (-acc / 2 + rd(d, p + (13918) - 234) + rd(d, p + (13682) - 154) / 2) as i16 as i32;
    wr(d, p + (13682), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (13682) - 154) * 3 / 2) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13526) - 2224) + rd(d, p + (11300) - 1092) / 2) as i16 as i32;
    wr(d, p + (11300), -acc);
    acc = (-acc / 2 + rd(d, p + (11300) - 1092)) as i16 as i32;
    wr(d, p + (10206), acc);
    acc = (rd(d, p + (10206) - 2610) * 3 / 4 + rd(d, p + (7594) - 1271) / 2) as i16 as i32;
    wr(d, p + (7594), -acc);
    acc = (-acc / 2 + rd(d, p + (7594) - 1271)) as i16 as i32;
    wr(d, p + (6321), acc);
    acc = (rd(d, p + (6321) - 3357) / 2 + rd(d, p + (2962) - 1112) / 2) as i16 as i32;
    wr(d, p + (2962), -acc);
    acc = (-acc / 2 + rd(d, p + (2962) - 1112) + rd(d, p + (1845) - 1) / 2) as i16 as i32;
    wr(d, p + (1845), acc);
    acc = (rd(d, p + (1845) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13526), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_27(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (15922) - 420) * 9 / 8 + rd(d, p + (14952) - 334) * 9 / 8 + rd(d, p + (13863) - 252) * 9 / 8 + rd(d, p + (12452) - 478) * 9 / 8 + rd(d, p + (8583) - 369) * 9 / 8 + rd(d, p + (7708) - 499) * 9 / 8 + rd(d, p + (6472) - 323) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 129) * 9 / 8 + rd(d, p + (14952) - 130) * 9 / 8 + rd(d, p + (13863) - 459) * 9 / 8 + rd(d, p + (12452) - 277) * 9 / 8 + rd(d, p + (8583) - 363) * 9 / 8 + rd(d, p + (7708) - 208) * 9 / 8 + rd(d, p + (5025) - 491) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 19) * 3 / 4 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16360) - 45) * 3 / 4 + rd(d, p + (16381) - 19)) as i16 as i32;
    wr(d, p + (16360), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16313) - 84) * 3 / 4 + rd(d, p + (16360) - 45)) as i16 as i32;
    wr(d, p + (16313), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16227) - 125) * 3 / 4 + rd(d, p + (16313) - 84)) as i16 as i32;
    wr(d, p + (16227), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176) * 3 / 4 + rd(d, p + (16227) - 125)) as i16 as i32;
    wr(d, p + (16100), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (16100) - 176)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (15922) - 968) + rd(d, p + (11046) - 1056) / 2) as i16 as i32;
    wr(d, p + (11046), -acc);
    acc = (-acc / 2 + rd(d, p + (11046) - 1056)) as i16 as i32;
    wr(d, p + (14952), acc);
    acc = (rd(d, p + (14952) - 1087) / 2 + rd(d, p + (14952) - 1088) / 2 + rd(d, p + (9988) - 612) / 2) as i16 as i32;
    wr(d, p + (9988), -acc);
    acc = (-acc / 2 + rd(d, p + (9988) - 612)) as i16 as i32;
    wr(d, p + (13863), acc);
    acc = (rd(d, p + (13863) - 1409) * 3 / 4 + rd(d, p + (9374) - 356) / 2) as i16 as i32;
    wr(d, p + (9374), -acc);
    acc = (-acc / 2 + rd(d, p + (9374) - 356)) as i16 as i32;
    wr(d, p + (12452), acc);
    acc = (rd(d, p + (12452) - 1404) * 3 / 4 + rd(d, p + (9016) - 431) / 2) as i16 as i32;
    wr(d, p + (9016), -acc);
    acc = (-acc / 2 + rd(d, p + (9016) - 431) + tmp_b) as i16 as i32;
    wr(d, p + (15922), acc);
    acc = (rd(d, p + (8583) - 873) + rd(d, p + (3413) - 987) / 2) as i16 as i32;
    wr(d, p + (3413), -acc);
    acc = (-acc / 2 + rd(d, p + (3413) - 987)) as i16 as i32;
    wr(d, p + (7708), acc);
    acc = (rd(d, p + (7708) - 1235) / 2 + rd(d, p + (2424) - 681) / 2) as i16 as i32;
    wr(d, p + (2424), -acc);
    acc = (-acc / 2 + rd(d, p + (2424) - 681)) as i16 as i32;
    wr(d, p + (6472), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (6472) - 1445) * 3 / 4 + rd(d, p + (1741) - 634) / 2) as i16 as i32;
    wr(d, p + (1741), -acc);
    acc = (-acc / 2 + rd(d, p + (1741) - 634)) as i16 as i32;
    wr(d, p + (5025), acc);
    acc = (rd(d, p + (5025) - 1610) * 3 / 4 + rd(d, p + (1105) - 432) / 2) as i16 as i32;
    wr(d, p + (1105), -acc);
    acc = (-acc / 2 + rd(d, p + (1105) - 432) + tmp_b) as i16 as i32;
    wr(d, p + (8583), acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_28(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (13341) - 21) * 9 / 8 + rd(d, p + (9321) - 1891) * 9 / 8 + rd(d, p + (5236) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (13341) - 1010) * 9 / 8 + rd(d, p + (9321) - 890) * 9 / 8 + rd(d, p + (5236) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 321) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14363) - 1020) / 2) as i16 as i32;
    wr(d, p + (14363), -acc);
    acc = (-acc / 2 + rd(d, p + (14363) - 1020) + rd(d, p + (14655) - 23) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 23) + rd(d, p + (14630) - 56) / 2) as i16 as i32;
    wr(d, p + (14630), -acc);
    acc = (-acc / 2 + rd(d, p + (14630) - 56) + rd(d, p + (14572) - 78) / 2) as i16 as i32;
    wr(d, p + (14572), -acc);
    acc = (-acc / 2 + rd(d, p + (14572) - 78) + rd(d, p + (14492) - 127) / 2) as i16 as i32;
    wr(d, p + (14492), -acc);
    acc = (-acc / 2 + rd(d, p + (14492) - 127)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (13341) - 2624) / 2 + rd(d, p + (13341) - 2625) / 2 + rd(d, p + (10715) - 1392) / 2) as i16 as i32;
    wr(d, p + (10715), -acc);
    acc = (-acc / 2 + rd(d, p + (10715) - 1392)) as i16 as i32;
    wr(d, p + (9321), acc);
    acc = (rd(d, p + (9321) - 2810) * 15 / 16 + rd(d, p + (6509) - 1271) / 2) as i16 as i32;
    wr(d, p + (6509), -acc);
    acc = (-acc / 2 + rd(d, p + (6509) - 1271)) as i16 as i32;
    wr(d, p + (5236), acc);
    acc = (rd(d, p + (5236) - 3857) * 3 / 4 + rd(d, p + (1377) - 1112) / 2) as i16 as i32;
    wr(d, p + (1377), -acc);
    acc = (-acc / 2 + rd(d, p + (1377) - 1112) + rd(d, p + (260) - 1) / 2) as i16 as i32;
    wr(d, p + (260), acc);
    acc = (rd(d, p + (260) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (13341), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_29(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12741) - 21) * 9 / 8 + rd(d, p + (9121) - 1891) * 9 / 8 + rd(d, p + (4636) - 2003) * 9 / 8) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (12741) - 1010) * 9 / 8 + rd(d, p + (9121) - 890) * 9 / 8 + rd(d, p + (4636) - 671) * 9 / 8) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16381) - 321) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 321) + rd(d, p + (16058) - 431) / 2) as i16 as i32;
    wr(d, p + (16058), -acc);
    acc = (-acc / 2 + rd(d, p + (16058) - 431) + rd(d, p + (15625) - 968) / 2) as i16 as i32;
    wr(d, p + (15625), -acc);
    acc = (-acc / 2 + rd(d, p + (15625) - 968) + rd(d, p + (14363) - 1620) / 2) as i16 as i32;
    wr(d, p + (14363), -acc);
    acc = (-acc / 2 + rd(d, p + (14363) - 1620) + rd(d, p + (14655) - 23) / 2) as i16 as i32;
    wr(d, p + (14655), -acc);
    acc = (-acc / 2 + rd(d, p + (14655) - 23) + rd(d, p + (14630) - 56) / 2) as i16 as i32;
    wr(d, p + (14630), -acc);
    acc = (-acc / 2 + rd(d, p + (14630) - 56) + rd(d, p + (14572) - 78) / 2) as i16 as i32;
    wr(d, p + (14572), -acc);
    acc = (-acc / 2 + rd(d, p + (14572) - 78) + rd(d, p + (14492) - 127) / 2) as i16 as i32;
    wr(d, p + (14492), -acc);
    acc = (-acc / 2 + rd(d, p + (14492) - 127)) as i16 as i32;
    tmp_b = (acc) as i16 as i32;
    acc = (rd(d, p + (12741) - 1624) / 2 + rd(d, p + (12741) - 1625) / 2 + rd(d, p + (11115) - 1992) / 2) as i16 as i32;
    wr(d, p + (11115), -acc);
    acc = (-acc / 2 + rd(d, p + (11115) - 1992)) as i16 as i32;
    wr(d, p + (9121), acc);
    acc = (rd(d, p + (9121) - 2110) * 15 / 16 + rd(d, p + (7009) - 2371) / 2) as i16 as i32;
    wr(d, p + (7009), -acc);
    acc = (-acc / 2 + rd(d, p + (7009) - 2371)) as i16 as i32;
    wr(d, p + (4636), acc);
    acc = (rd(d, p + (4636) - 2157) * 15 / 16 + rd(d, p + (2477) - 2212) / 2) as i16 as i32;
    wr(d, p + (2477), -acc);
    acc = (-acc / 2 + rd(d, p + (2477) - 2212) + rd(d, p + (260) - 1) / 2) as i16 as i32;
    wr(d, p + (260), acc);
    acc = (rd(d, p + (260) - 0) / 2 + tmp_b) as i16 as i32;
    wr(d, p + (12741), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_30(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_15 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (6109) - 5) / 2 + rd(d, p + (5375) - 361) / 2 + rd(d, p + (4072) - 120) / 2 + rd(d, p + (3222) - 861) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 126) / 2 + rd(d, p + (5375) - 897) / 2 + rd(d, p + (4072) - 1) / 2 + rd(d, p + (3222) - 356) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 0) / 2 + rd(d, p + (0) - 245) / 2 + rd(d, p + (0) - 523) / 2 + rd(d, p + (0) - 891) / 2 + rd(d, p + (0) - 1200) / 2 + rd(d, p + (0) - 1568) / 2 + rd(d, p + (0) - 1987) / 2 + rd(d, p + (0) - 2567) / 2 + rd(d, p + (0) - 2893) / 2 + rd(d, p + (0) - 3104) / 2) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (-acc / 8 + rd(d, p + (7382) - 245) * 5 / 8 + tmp_b / 2) as i16 as i32;
    wr(d, p + (7382), -acc);
    acc = (-acc / 8 + rd(d, p + (7135) - 189) * 5 / 8 + rd(d, p + (7382) - 0) / 2 + rd(d, p + (7382) - 245)) as i16 as i32;
    wr(d, p + (7135), -acc);
    acc = (-acc / 8 + rd(d, p + (6944) - 145) * 5 / 8 + rd(d, p + (7135) - 0) / 2 + rd(d, p + (7135) - 189)) as i16 as i32;
    wr(d, p + (6944), -acc);
    acc = (-acc / 8 + rd(d, p + (6797) - 123) * 5 / 8 + rd(d, p + (6944) - 0) / 2 + rd(d, p + (6944) - 145)) as i16 as i32;
    wr(d, p + (6797), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 198) * 5 / 8 + rd(d, p + (6797) - 0) / 2 + rd(d, p + (6797) - 123)) as i16 as i32;
    wr(d, p + (6672), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 0) / 2 + rd(d, p + (6672) - 198) + rd(d, p + (6472) - 34) / 2) as i16 as i32;
    wr(d, p + (6472), -acc);
    acc = (-acc / 2 + rd(d, p + (6472) - 34) + rd(d, p + (6436) - 69) / 2) as i16 as i32;
    wr(d, p + (6436), -acc);
    acc = (-acc / 2 + rd(d, p + (6436) - 69) + rd(d, p + (6365) - 104) / 2) as i16 as i32;
    wr(d, p + (6365), -acc);
    acc = (-acc / 2 + rd(d, p + (6365) - 104) + rd(d, p + (6259) - 148) / 2) as i16 as i32;
    wr(d, p + (6259), -acc);
    acc = (-acc / 2 + rd(d, p + (6259) - 148)) as i16 as i32;
    tmp_15 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 497) / 2 + rd(d, p + (5610) - 233) / 2) as i16 as i32;
    wr(d, p + (5610), -acc);
    acc = (-acc / 2 + rd(d, p + (5610) - 233)) as i16 as i32;
    wr(d, p + (5375), acc);
    acc = (rd(d, p + (5375) - 1010) * 3 / 4 + rd(d, p + (4363) - 289) / 2) as i16 as i32;
    wr(d, p + (4363), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (4363) - 289) + tmp_15) as i16 as i32;
    wr(d, p + (4072), -acc);
    acc = (rd(d, p + (4072) - 590) / 2 + rd(d, p + (3480) - 256) / 2) as i16 as i32;
    wr(d, p + (3480), -acc);
    acc = (-acc / 2 + rd(d, p + (3480) - 256)) as i16 as i32;
    wr(d, p + (3222), acc);
    acc = (rd(d, p + (3222) - 1071) * 3 / 4 + rd(d, p + (2149) - 225) / 2) as i16 as i32;
    wr(d, p + (2149), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (2149) - 225) + tmp_15) as i16 as i32;
    wr(d, p + (6109), -acc);
    (left, right)
}

fn midiverb2_effect_31(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_15 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (6109) - 123) / 2 + rd(d, p + (4749) - 361) / 2 + rd(d, p + (3446) - 205) / 2 + rd(d, p + (2196) - 861) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 126) / 2 + rd(d, p + (4749) - 897) / 2 + rd(d, p + (3446) - 202) / 2 + rd(d, p + (2196) - 356) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 0) / 2 + rd(d, p + (0) - 245) / 2 + rd(d, p + (0) - 891) / 2 + rd(d, p + (0) - 1568) / 2 + rd(d, p + (0) - 1987) / 2 + rd(d, p + (0) - 2746) / 2 + rd(d, p + (0) - 3245) / 2) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (-acc / 8 + rd(d, p + (7382) - 245) * 5 / 8 + tmp_b / 2) as i16 as i32;
    wr(d, p + (7382), -acc);
    acc = (-acc / 8 + rd(d, p + (7135) - 189) * 5 / 8 + rd(d, p + (7382) - 0) / 2 + rd(d, p + (7382) - 245)) as i16 as i32;
    wr(d, p + (7135), -acc);
    acc = (-acc / 8 + rd(d, p + (6944) - 145) * 5 / 8 + rd(d, p + (7135) - 0) / 2 + rd(d, p + (7135) - 189)) as i16 as i32;
    wr(d, p + (6944), -acc);
    acc = (-acc / 8 + rd(d, p + (6797) - 123) * 5 / 8 + rd(d, p + (6944) - 0) / 2 + rd(d, p + (6944) - 145)) as i16 as i32;
    wr(d, p + (6797), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 198) * 5 / 8 + rd(d, p + (6797) - 0) / 2 + rd(d, p + (6797) - 123)) as i16 as i32;
    wr(d, p + (6672), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 0) / 2 + rd(d, p + (6672) - 198) + rd(d, p + (6472) - 34) / 2) as i16 as i32;
    wr(d, p + (6472), -acc);
    acc = (-acc / 2 + rd(d, p + (6472) - 34) + rd(d, p + (6436) - 69) / 2) as i16 as i32;
    wr(d, p + (6436), -acc);
    acc = (-acc / 2 + rd(d, p + (6436) - 69) + rd(d, p + (6365) - 104) / 2) as i16 as i32;
    wr(d, p + (6365), -acc);
    acc = (-acc / 2 + rd(d, p + (6365) - 104) + rd(d, p + (6259) - 148) / 2) as i16 as i32;
    wr(d, p + (6259), -acc);
    acc = (-acc / 2 + rd(d, p + (6259) - 148)) as i16 as i32;
    tmp_15 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 1123) * 3 / 4 + rd(d, p + (4984) - 233) / 2) as i16 as i32;
    wr(d, p + (4984), -acc);
    acc = (-acc / 2 + rd(d, p + (4984) - 233)) as i16 as i32;
    wr(d, p + (4749), acc);
    acc = (-rd(d, p + (4749) - 1010) / 4 + -rd(d, p + (4749) - 1011) / 4 + rd(d, p + (3737) - 289) / 2) as i16 as i32;
    wr(d, p + (3737), -acc);
    acc = (-acc / 2 + rd(d, p + (3737) - 289) + tmp_15) as i16 as i32;
    wr(d, p + (6109), -acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (3446) - 990) * 3 / 4 + rd(d, p + (2454) - 256) / 2) as i16 as i32;
    wr(d, p + (2454), -acc);
    acc = (-acc / 2 + rd(d, p + (2454) - 256)) as i16 as i32;
    wr(d, p + (2196), acc);
    acc = (-rd(d, p + (2196) - 1371) / 4 + -rd(d, p + (2196) - 1372) / 4 + rd(d, p + (823) - 225) / 2) as i16 as i32;
    wr(d, p + (823), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (823) - 225) + tmp_15) as i16 as i32;
    wr(d, p + (3446), -acc);
    (left, right)
}

fn midiverb2_effect_32(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_15 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (8865) - 123) / 2 + rd(d, p + (7228) - 361) / 2 + rd(d, p + (5925) - 505) / 2 + rd(d, p + (4645) - 861) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (8865) - 626) / 2 + rd(d, p + (7228) - 897) / 2 + rd(d, p + (5925) - 56) / 2 + rd(d, p + (4645) - 756) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1000) / 2 + rd(d, p + (0) - 999) / 2) as i16 as i32;
    wr(d, p + (15384), acc);
    acc = (rd(d, p + (15384) - 4423) / 2 + rd(d, p + (15384) - 3987) / 2 + rd(d, p + (15384) - 3231) / 2 + rd(d, p + (15384) - 2889) / 2 + rd(d, p + (15384) - 2221) / 2 + rd(d, p + (15384) - 1746) / 2 + rd(d, p + (15384) - 1341) / 2 + rd(d, p + (15384) - 923) / 2 + rd(d, p + (15384) - 523) / 2 + rd(d, p + (15384) - 109) / 2 + rd(d, p + (0) - 876) / 2 + rd(d, p + (0) - 435) / 2 + rd(d, p + (0) - 235) / 2 + rd(d, p + (0) - 0) / 2) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (-acc / 8 + rd(d, p + (10382) - 245) * 5 / 8 + tmp_b / 2) as i16 as i32;
    wr(d, p + (10382), -acc);
    acc = (-acc / 8 + rd(d, p + (10135) - 189) * 5 / 8 + rd(d, p + (10382) - 0) / 2 + rd(d, p + (10382) - 245)) as i16 as i32;
    wr(d, p + (10135), -acc);
    acc = (-acc / 8 + rd(d, p + (9944) - 189) * 5 / 8 + rd(d, p + (10135) - 0) / 2 + rd(d, p + (10135) - 189)) as i16 as i32;
    wr(d, p + (9944), -acc);
    acc = (-acc / 8 + rd(d, p + (9753) - 223) * 5 / 8 + rd(d, p + (9944) - 0) / 2 + rd(d, p + (9944) - 189)) as i16 as i32;
    wr(d, p + (9753), -acc);
    acc = (-acc / 8 + rd(d, p + (9528) - 298) * 5 / 8 + rd(d, p + (9753) - 0) / 2 + rd(d, p + (9753) - 223)) as i16 as i32;
    wr(d, p + (9528), -acc);
    acc = (-acc / 8 + rd(d, p + (9528) - 0) / 2 + rd(d, p + (9528) - 298) + rd(d, p + (9228) - 34) / 2) as i16 as i32;
    wr(d, p + (9228), -acc);
    acc = (-acc / 2 + rd(d, p + (9228) - 34) + rd(d, p + (9192) - 69) / 2) as i16 as i32;
    wr(d, p + (9192), -acc);
    acc = (-acc / 2 + rd(d, p + (9192) - 69) + rd(d, p + (9121) - 104) / 2) as i16 as i32;
    wr(d, p + (9121), -acc);
    acc = (-acc / 2 + rd(d, p + (9121) - 104) + rd(d, p + (9015) - 148) / 2) as i16 as i32;
    wr(d, p + (9015), -acc);
    acc = (-acc / 2 + rd(d, p + (9015) - 148)) as i16 as i32;
    tmp_15 = (-acc) as i16 as i32;
    acc = (rd(d, p + (8865) - 1310) / 2 + rd(d, p + (7553) - 323) / 2) as i16 as i32;
    wr(d, p + (7553), -acc);
    acc = (-acc / 2 + rd(d, p + (7553) - 323)) as i16 as i32;
    wr(d, p + (7228), acc);
    left = (tmp_5) as i16;
    acc = (rd(d, p + (7228) - 1010) / 2 + rd(d, p + (6216) - 289) / 2) as i16 as i32;
    wr(d, p + (6216), -acc);
    acc = (-acc / 2 + rd(d, p + (6216) - 289) + tmp_15) as i16 as i32;
    wr(d, p + (5925), -acc);
    acc = (rd(d, p + (5925) - 1020) / 2) as i16 as i32;
    wr(d, p + (4903), -acc);
    acc = (-acc / 2 + rd(d, p + (4903) - 256)) as i16 as i32;
    wr(d, p + (4645), acc);
    acc = (rd(d, p + (4645) - 1071) / 2 + rd(d, p + (3572) - 125) / 2) as i16 as i32;
    wr(d, p + (3572), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (3572) - 125) + tmp_15) as i16 as i32;
    wr(d, p + (8865), -acc);
    (left, right)
}

fn midiverb2_effect_33(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_15 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (6259) - 123) / 2 + rd(d, p + (4899) - 361) / 2 + rd(d, p + (3596) - 205) / 2 + rd(d, p + (2346) - 861) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (6259) - 126) / 2 + rd(d, p + (4899) - 897) / 2 + rd(d, p + (3596) - 202) / 2 + rd(d, p + (2346) - 356) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 0) / 2 + rd(d, p + (0) - 245) / 2 + rd(d, p + (0) - 891) / 2 + rd(d, p + (0) - 1568) / 2 + rd(d, p + (0) - 1987) / 2 + rd(d, p + (0) - 2746) / 2 + rd(d, p + (0) - 3245) / 2 + rd(d, p + (0) - 3678) / 2 + rd(d, p + (0) - 4232) / 2 + rd(d, p + (0) - 4980) / 2 + rd(d, p + (0) - 5123) / 2 + rd(d, p + (0) - 5765) / 2 + rd(d, p + (0) - 6345) / 2 + rd(d, p + (0) - 7045) / 2 + rd(d, p + (0) - 7589) / 2 + rd(d, p + (0) - 8245) / 2 + rd(d, p + (0) - 8531) / 2 + rd(d, p + (0) - 8921) / 2) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (-acc / 8 + rd(d, p + (7382) - 245) * 5 / 8 + tmp_b / 2) as i16 as i32;
    wr(d, p + (7382), -acc);
    acc = (-acc / 8 + rd(d, p + (7135) - 189) * 5 / 8 + rd(d, p + (7382) - 0) / 2 + rd(d, p + (7382) - 245)) as i16 as i32;
    wr(d, p + (7135), -acc);
    acc = (-acc / 8 + rd(d, p + (6944) - 145) * 5 / 8 + rd(d, p + (7135) - 0) / 2 + rd(d, p + (7135) - 189)) as i16 as i32;
    wr(d, p + (6944), -acc);
    acc = (-acc / 8 + rd(d, p + (6797) - 123) * 5 / 8 + rd(d, p + (6944) - 0) / 2 + rd(d, p + (6944) - 145)) as i16 as i32;
    wr(d, p + (6797), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 198) * 5 / 8 + rd(d, p + (6797) - 0) / 2 + rd(d, p + (6797) - 123)) as i16 as i32;
    wr(d, p + (6672), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 0) / 2 + rd(d, p + (6672) - 198) + rd(d, p + (6472) - 34) / 2) as i16 as i32;
    wr(d, p + (6472), -acc);
    acc = (-acc / 2 + rd(d, p + (6472) - 34) + rd(d, p + (6436) - 69) / 2) as i16 as i32;
    wr(d, p + (6436), -acc);
    acc = (-acc / 2 + rd(d, p + (6436) - 69) + rd(d, p + (6365) - 104) / 2) as i16 as i32;
    wr(d, p + (6365), -acc);
    acc = (-acc / 2 + rd(d, p + (6365) - 104)) as i16 as i32;
    tmp_15 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6259) - 1123) / 2 + rd(d, p + (5134) - 233) / 2) as i16 as i32;
    wr(d, p + (5134), -acc);
    acc = (-acc / 2 + rd(d, p + (5134) - 233)) as i16 as i32;
    wr(d, p + (4899), acc);
    acc = (rd(d, p + (4899) - 1010) / 2 + rd(d, p + (3887) - 289) / 2) as i16 as i32;
    wr(d, p + (3887), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (3887) - 289) + rd(d, p + (746) - 1) / 2) as i16 as i32;
    wr(d, p + (746), acc);
    acc = (acc * -3 / 4 + tmp_15) as i16 as i32;
    wr(d, p + (6259), -acc);
    acc = (rd(d, p + (3596) - 990) / 2 + rd(d, p + (2604) - 256) / 2) as i16 as i32;
    wr(d, p + (2604), -acc);
    acc = (-acc / 2 + rd(d, p + (2604) - 256)) as i16 as i32;
    wr(d, p + (2346), acc);
    right = (tmp_3) as i16;
    acc = (rd(d, p + (2346) - 1371) / 2 + rd(d, p + (973) - 225) / 2) as i16 as i32;
    wr(d, p + (973), -acc);
    acc = (-acc / 2 + rd(d, p + (973) - 225) + rd(d, p + (743) - 1) / 2) as i16 as i32;
    wr(d, p + (743), acc);
    acc = (acc * -3 / 4 + tmp_15) as i16 as i32;
    wr(d, p + (3596), -acc);
    (left, right)
}

fn midiverb2_effect_34(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_b = 0i32;
    let mut tmp_15 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (6109) - 123) / 2 + rd(d, p + (4749) - 361) / 2 + rd(d, p + (3446) - 205) / 2 + rd(d, p + (2196) - 861) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 126) / 2 + rd(d, p + (4749) - 897) / 2 + rd(d, p + (3446) - 202) / 2 + rd(d, p + (2196) - 356) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 0) / 2 + rd(d, p + (0) - 245) / 2 + rd(d, p + (0) - 891) / 2 + rd(d, p + (0) - 1568) / 2 + rd(d, p + (0) - 1987) / 2 + rd(d, p + (0) - 2746) / 2 + rd(d, p + (0) - 3245) / 2 + rd(d, p + (0) - 3678) / 2 + rd(d, p + (0) - 4232) / 2 + rd(d, p + (0) - 4980) / 2 + rd(d, p + (0) - 5123) / 2 + rd(d, p + (0) - 5765) / 2 + rd(d, p + (0) - 6345) / 2 + rd(d, p + (0) - 7045) / 2 + rd(d, p + (0) - 7589) / 2 + rd(d, p + (0) - 8245) / 2 + rd(d, p + (0) - 8531) / 2 + rd(d, p + (0) - 8921) / 2) as i16 as i32;
    tmp_b = (-acc) as i16 as i32;
    acc = (-acc / 8 + rd(d, p + (7382) - 245) * 5 / 8 + tmp_b / 2) as i16 as i32;
    wr(d, p + (7382), -acc);
    acc = (-acc / 8 + rd(d, p + (7135) - 189) * 5 / 8 + rd(d, p + (7382) - 0) / 2 + rd(d, p + (7382) - 245)) as i16 as i32;
    wr(d, p + (7135), -acc);
    acc = (-acc / 8 + rd(d, p + (6944) - 145) * 5 / 8 + rd(d, p + (7135) - 0) / 2 + rd(d, p + (7135) - 189)) as i16 as i32;
    wr(d, p + (6944), -acc);
    acc = (-acc / 8 + rd(d, p + (6797) - 123) * 5 / 8 + rd(d, p + (6944) - 0) / 2 + rd(d, p + (6944) - 145)) as i16 as i32;
    wr(d, p + (6797), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 198) * 5 / 8 + rd(d, p + (6797) - 0) / 2 + rd(d, p + (6797) - 123)) as i16 as i32;
    wr(d, p + (6672), -acc);
    acc = (-acc / 8 + rd(d, p + (6672) - 0) / 2 + rd(d, p + (6672) - 198) + rd(d, p + (6472) - 34) / 2) as i16 as i32;
    wr(d, p + (6472), -acc);
    acc = (-acc / 2 + rd(d, p + (6472) - 34) + rd(d, p + (6436) - 69) / 2) as i16 as i32;
    wr(d, p + (6436), -acc);
    acc = (-acc / 2 + rd(d, p + (6436) - 69) + rd(d, p + (6365) - 104) / 2) as i16 as i32;
    wr(d, p + (6365), -acc);
    acc = (-acc / 2 + rd(d, p + (6365) - 104) + rd(d, p + (6259) - 148) / 2) as i16 as i32;
    wr(d, p + (6259), -acc);
    acc = (-acc / 2 + rd(d, p + (6259) - 148)) as i16 as i32;
    tmp_15 = (-acc) as i16 as i32;
    acc = (rd(d, p + (6109) - 1123) * 3 / 4 + rd(d, p + (4984) - 233) / 2) as i16 as i32;
    wr(d, p + (4984), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (4984) - 233)) as i16 as i32;
    wr(d, p + (4749), acc);
    acc = (-rd(d, p + (4749) - 1010) / 4 + -rd(d, p + (4749) - 1011) / 4 + rd(d, p + (3737) - 289) / 2) as i16 as i32;
    wr(d, p + (3737), -acc);
    acc = (-acc / 2 + rd(d, p + (3737) - 289) + tmp_15) as i16 as i32;
    wr(d, p + (3446), -acc);
    acc = (rd(d, p + (3446) - 990) * 3 / 4 + rd(d, p + (2454) - 256) / 2) as i16 as i32;
    wr(d, p + (2454), -acc);
    right = (tmp_3) as i16;
    acc = (-acc / 2 + rd(d, p + (2454) - 256)) as i16 as i32;
    wr(d, p + (2196), acc);
    acc = (-rd(d, p + (2196) - 1371) / 4 + -rd(d, p + (2196) - 1372) / 4 + rd(d, p + (823) - 225) / 2) as i16 as i32;
    wr(d, p + (823), -acc);
    acc = (-acc / 2 + rd(d, p + (823) - 225) + tmp_15) as i16 as i32;
    wr(d, p + (6109), -acc);
    (left, right)
}

fn midiverb2_effect_35(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) + rd(d, p + (2298) - 31) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 31) + rd(d, p + (2265) - 153) / 2) as i16 as i32;
    wr(d, p + (2265), -acc);
    acc = (-acc / 2 + rd(d, p + (2265) - 153) + rd(d, p + (2110) - 273) / 2) as i16 as i32;
    wr(d, p + (2110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (2110) - 273) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) + rd(d, p + (1835) - 42) / 2) as i16 as i32;
    wr(d, p + (1835), -acc);
    acc = (-acc / 2 + rd(d, p + (1835) - 42) + rd(d, p + (1791) - 181) / 2) as i16 as i32;
    wr(d, p + (1791), -acc);
    acc = (-acc / 2 + rd(d, p + (1791) - 181) + rd(d, p + (1608) - 233) / 2) as i16 as i32;
    wr(d, p + (1608), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (1608) - 233) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 0) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (0), acc);
    acc = (rd(d, p + (0) - 10) * -3 / 8) as i16 as i32;
    wr(d, p + (16373), -acc);
    acc = (rd(d, p + (16373) - 220) / 2 + rd(d, p + (16373) - 401) / 2 + rd(d, p + (16373) - 523) / 2 + rd(d, p + (16373) - 750) / 2 + rd(d, p + (16373) - 864) / 2 + rd(d, p + (15383) - 64) / 2 + rd(d, p + (15383) - 118) / 2 + rd(d, p + (15383) - 232) / 2 + rd(d, p + (15383) - 433) / 2 + rd(d, p + (15383) - 554) / 2 + rd(d, p + (15383) - 741) / 2 + rd(d, p + (15383) - 861) / 2 + rd(d, p + (15383) - 1045) / 2 + rd(d, p + (15383) - 1148) / 2 + rd(d, p + (15383) - 1255) / 2 + rd(d, p + (15383) - 1442) / 2 + rd(d, p + (15383) - 1578) / 2 + rd(d, p + (15383) - 1720) / 2 + rd(d, p + (15383) - 1872) / 2 + rd(d, p + (15383) - 2035) / 2 + rd(d, p + (15383) - 2120) / 2 + rd(d, p + (15383) - 2266) / 2 + rd(d, p + (15383) - 2450) / 2 + rd(d, p + (15383) - 2568) / 2 + rd(d, p + (15383) - 2724) / 2 + rd(d, p + (15383) - 2888) / 2 + rd(d, p + (15383) - 3033) / 2 + rd(d, p + (15383) - 3151) / 2 + rd(d, p + (15383) - 3256) / 2 + rd(d, p + (15383) - 3491) / 2 + rd(d, p + (15383) - 3555) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    acc = (-rd(d, p + (15383) - 8094) / 16 + -rd(d, p + (15383) - 7848) / 16 + -rd(d, p + (15383) - 7792) / 16 + -rd(d, p + (15383) - 7514) / 16 + -rd(d, p + (15383) - 7461) / 16 + -rd(d, p + (15383) - 7378) / 16 + -rd(d, p + (15383) - 7138) / 16 + -rd(d, p + (15383) - 7091) / 16 + rd(d, p + (15383) - 6864) / 8 + rd(d, p + (15383) - 6726) / 8 + rd(d, p + (15383) - 6656) / 8 + rd(d, p + (15383) - 6418) / 8 + rd(d, p + (15383) - 6363) / 8 + rd(d, p + (15383) - 6153) / 8 + rd(d, p + (15383) - 6017) / 8 + rd(d, p + (15383) - 5966) / 8 + -rd(d, p + (15383) - 5727) / 4 + -rd(d, p + (15383) - 5656) / 4 + -rd(d, p + (15383) - 5494) / 4 + -rd(d, p + (15383) - 5318) / 4 + -rd(d, p + (15383) - 5260) / 4 + -rd(d, p + (15383) - 5018) / 4 + -rd(d, p + (15383) - 4957) / 4 + -rd(d, p + (15383) - 4733) / 4 + rd(d, p + (15383) - 4626) / 2 + rd(d, p + (15383) - 4550) / 2 + rd(d, p + (15383) - 4338) / 2 + rd(d, p + (15383) - 4209) / 2 + rd(d, p + (15383) - 4045) / 2 + rd(d, p + (15383) - 3920) / 2 + rd(d, p + (15383) - 3799) / 2 + rd(d, p + (15383) - 3618) / 2 + rd(d, p + (1373) - 1) / 2) as i16 as i32;
    wr(d, p + (1373), acc);
    right = (tmp_3) as i16;
    acc = (-rd(d, p + (1373) - 1) / 4 + rd(d, p + (11) - 0)) as i16 as i32;
    wr(d, p + (11), -acc);
    acc = (rd(d, p + (16373) - 989) / 2 + rd(d, p + (1370) - 1) / 2) as i16 as i32;
    wr(d, p + (1370), -acc);
    acc = (rd(d, p + (1370) - 1)) as i16 as i32;
    wr(d, p + (15383), -acc);
    (left, right)
}

fn midiverb2_effect_36(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (1796), acc);
    acc = (rd(d, p + (1796) - 1500)) as i16 as i32;
    wr(d, p + (16383), -acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2220) - 173) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (1934) - 133) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16383) - 4) / 2 + rd(d, p + (16379) - 1) / 2) as i16 as i32;
    wr(d, p + (16379), acc);
    acc = (rd(d, p + (16379) - 5) / 2) as i16 as i32;
    wr(d, p + (16373), acc);
    acc = (rd(d, p + (16373) - 152) / 2 + rd(d, p + (16373) - 222) / 2 + rd(d, p + (16373) - 401) / 2 + rd(d, p + (16373) - 523) / 2 + rd(d, p + (16373) - 750) / 2 + rd(d, p + (16373) - 864) / 2 + rd(d, p + (16373) - 1054) / 2 + rd(d, p + (16373) - 1108) / 2 + rd(d, p + (16373) - 1222) / 2 + rd(d, p + (16373) - 1423) / 2 + rd(d, p + (16373) - 1544) / 2 + rd(d, p + (16373) - 1731) / 2 + rd(d, p + (16373) - 1851) / 2 + rd(d, p + (16373) - 2035) / 2 + rd(d, p + (16373) - 2138) / 2 + rd(d, p + (16373) - 2245) / 2 + rd(d, p + (16373) - 2432) / 2 + rd(d, p + (16373) - 2568) / 2 + rd(d, p + (16373) - 2710) / 2 + rd(d, p + (16373) - 2862) / 2 + rd(d, p + (16373) - 3025) / 2 + rd(d, p + (16373) - 3110) / 2 + rd(d, p + (16373) - 3256) / 2 + rd(d, p + (16373) - 3440) / 2 + rd(d, p + (16373) - 3558) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_37(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (rd(d, p + (11) - 2) + rd(d, p + (6298) - 31) / 2) as i16 as i32;
    wr(d, p + (6298), -acc);
    acc = (-acc / 2 + rd(d, p + (6298) - 31) + rd(d, p + (6265) - 153) / 2) as i16 as i32;
    wr(d, p + (6265), -acc);
    acc = (-acc / 2 + rd(d, p + (6265) - 153) + rd(d, p + (6110) - 273) / 2) as i16 as i32;
    wr(d, p + (6110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (6110) - 273) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) + rd(d, p + (5835) - 42) / 2) as i16 as i32;
    wr(d, p + (5835), -acc);
    acc = (-acc / 2 + rd(d, p + (5835) - 42) + rd(d, p + (5791) - 181) / 2) as i16 as i32;
    wr(d, p + (5791), -acc);
    acc = (-acc / 2 + rd(d, p + (5791) - 181) + rd(d, p + (5608) - 233) / 2) as i16 as i32;
    wr(d, p + (5608), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (5608) - 233) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (5364), -acc);
    acc = (rd(d, p + (5364) - 1500)) as i16 as i32;
    wr(d, p + (16382), -acc);
    acc = (rd(d, p + (16382) - 2) / 2 + rd(d, p + (16380) - 1) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    acc = (rd(d, p + (16380) - 6) * -3 / 8) as i16 as i32;
    wr(d, p + (16373), -acc);
    acc = (-acc / 2 + rd(d, p + (16373) - 523) / 2 + rd(d, p + (16373) - 750) / 2 + rd(d, p + (16373) - 864) / 2 + rd(d, p + (15383) - 64) / 2 + rd(d, p + (15383) - 118) / 2 + rd(d, p + (15383) - 232) / 2 + rd(d, p + (15383) - 433) / 2 + rd(d, p + (15383) - 554) / 2 + rd(d, p + (15383) - 741) / 2 + rd(d, p + (15383) - 861) / 2 + rd(d, p + (15383) - 1045) / 2 + rd(d, p + (15383) - 1148) / 2 + rd(d, p + (15383) - 1255) / 2 + rd(d, p + (15383) - 1442) / 2 + rd(d, p + (15383) - 1578) / 2 + rd(d, p + (15383) - 1720) / 2 + rd(d, p + (15383) - 1872) / 2 + rd(d, p + (15383) - 2035) / 2 + rd(d, p + (15383) - 2120) / 2 + rd(d, p + (15383) - 2266) / 2 + rd(d, p + (15383) - 2450) / 2 + rd(d, p + (15383) - 2568) / 2 + rd(d, p + (15383) - 2724) / 2 + rd(d, p + (15383) - 2888) / 2 + rd(d, p + (15383) - 3033) / 2 + rd(d, p + (15383) - 3151) / 2 + rd(d, p + (15383) - 3256) / 2 + rd(d, p + (15383) - 3491) / 2 + rd(d, p + (15383) - 3555) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (-rd(d, p + (15383) - 8094) / 16 + -rd(d, p + (15383) - 7848) / 16 + -rd(d, p + (15383) - 7792) / 16 + -rd(d, p + (15383) - 7514) / 16 + -rd(d, p + (15383) - 7461) / 16 + -rd(d, p + (15383) - 7378) / 16 + -rd(d, p + (15383) - 7138) / 16 + -rd(d, p + (15383) - 7091) / 16 + rd(d, p + (15383) - 6864) / 8 + rd(d, p + (15383) - 6726) / 8 + rd(d, p + (15383) - 6656) / 8 + rd(d, p + (15383) - 6418) / 8 + rd(d, p + (15383) - 6363) / 8 + rd(d, p + (15383) - 6153) / 8 + rd(d, p + (15383) - 6017) / 8 + rd(d, p + (15383) - 5966) / 8 + -rd(d, p + (15383) - 5727) / 4 + -rd(d, p + (15383) - 5656) / 4 + -rd(d, p + (15383) - 5494) / 4 + -rd(d, p + (15383) - 5318) / 4 + -rd(d, p + (15383) - 5260) / 4 + -rd(d, p + (15383) - 5018) / 4 + -rd(d, p + (15383) - 4957) / 4 + -rd(d, p + (15383) - 4733) / 4 + rd(d, p + (15383) - 4626) / 2 + rd(d, p + (15383) - 4550) / 2 + rd(d, p + (15383) - 4338) / 2 + rd(d, p + (15383) - 4209) / 2 + rd(d, p + (15383) - 4045) / 2 + rd(d, p + (15383) - 3920) / 2 + rd(d, p + (15383) - 3799) / 2 + rd(d, p + (15383) - 3618) / 2 + rd(d, p + (5373) - 1) / 2) as i16 as i32;
    wr(d, p + (5373), acc);
    acc = (-rd(d, p + (5373) - 1) / 4 + rd(d, p + (11) - 0)) as i16 as i32;
    wr(d, p + (11), -acc);
    acc = (rd(d, p + (16373) - 989) / 2 + rd(d, p + (5370) - 1) / 2) as i16 as i32;
    wr(d, p + (5370), -acc);
    acc = (rd(d, p + (5370) - 1)) as i16 as i32;
    wr(d, p + (15383), -acc);
    (left, right)
}

fn midiverb2_effect_38(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (2220) - 173) * 3 / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (acc * -3 / 4 + rd(d, p + (1934) - 133) * 3 / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (rd(d, p + (16382) - 1) * 3 / 4 + tmp_0 / 2) as i16 as i32;
    wr(d, p + (16382), acc);
    acc = (rd(d, p + (16382) - 8) / 2) as i16 as i32;
    wr(d, p + (16373), acc);
    acc = (rd(d, p + (16373) - 152) / 2 + rd(d, p + (16373) - 222) / 2 + rd(d, p + (16373) - 401) / 2 + rd(d, p + (16373) - 523) / 2 + rd(d, p + (16373) - 750) / 2 + rd(d, p + (16373) - 864) / 2 + rd(d, p + (16373) - 1054) / 2 + rd(d, p + (16373) - 1108) / 2 + rd(d, p + (16373) - 1222) / 2 + rd(d, p + (16373) - 1423) / 2 + rd(d, p + (16373) - 1544) / 2 + rd(d, p + (16373) - 1731) / 2 + rd(d, p + (16373) - 1851) / 2 + rd(d, p + (16373) - 2035) / 2 + rd(d, p + (16373) - 2138) / 2 + rd(d, p + (16373) - 2245) / 2 + rd(d, p + (16373) - 2432) / 2 + rd(d, p + (16373) - 2568) / 2 + rd(d, p + (16373) - 2710) / 2 + rd(d, p + (16373) - 2862) / 2 + rd(d, p + (16373) - 3025) / 2 + rd(d, p + (16373) - 3110) / 2 + rd(d, p + (16373) - 3256) / 2 + rd(d, p + (16373) - 3440) / 2 + rd(d, p + (16373) - 3558) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_39(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (rd(d, p + (11) - 2) + rd(d, p + (2298) - 31) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 31) + rd(d, p + (2265) - 153) / 2) as i16 as i32;
    wr(d, p + (2265), -acc);
    acc = (-acc / 2 + rd(d, p + (2265) - 153) + rd(d, p + (2110) - 273) / 2) as i16 as i32;
    wr(d, p + (2110), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (2110) - 273) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) + rd(d, p + (1835) - 42) / 2) as i16 as i32;
    wr(d, p + (1835), -acc);
    acc = (-acc / 2 + rd(d, p + (1835) - 42) + rd(d, p + (1791) - 181) / 2) as i16 as i32;
    wr(d, p + (1791), -acc);
    acc = (-acc / 2 + rd(d, p + (1791) - 181) + rd(d, p + (1608) - 233) / 2) as i16 as i32;
    wr(d, p + (1608), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (1608) - 233) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16382) - 1) * 3 / 4 + tmp_0 / 2) as i16 as i32;
    wr(d, p + (16382), acc);
    acc = (rd(d, p + (16382) - 8) * -3 / 8) as i16 as i32;
    wr(d, p + (16373), -acc);
    acc = (rd(d, p + (16373) - 220) / 2 + rd(d, p + (16373) - 401) / 2 + rd(d, p + (16373) - 523) / 2 + rd(d, p + (16373) - 750) / 2 + rd(d, p + (16373) - 864) / 2 + rd(d, p + (15383) - 64) / 2 + rd(d, p + (15383) - 118) / 2 + rd(d, p + (15383) - 232) / 2 + rd(d, p + (15383) - 433) / 2 + rd(d, p + (15383) - 554) / 2 + rd(d, p + (15383) - 741) / 2 + rd(d, p + (15383) - 861) / 2 + rd(d, p + (15383) - 1045) / 2 + rd(d, p + (15383) - 1148) / 2 + rd(d, p + (15383) - 1255) / 2 + rd(d, p + (15383) - 1442) / 2 + rd(d, p + (15383) - 1578) / 2 + rd(d, p + (15383) - 1720) / 2 + rd(d, p + (15383) - 1872) / 2 + rd(d, p + (15383) - 2035) / 2 + rd(d, p + (15383) - 2120) / 2 + rd(d, p + (15383) - 2266) / 2 + rd(d, p + (15383) - 2450) / 2 + rd(d, p + (15383) - 2568) / 2 + rd(d, p + (15383) - 2724) / 2 + rd(d, p + (15383) - 2888) / 2 + rd(d, p + (15383) - 3033) / 2 + rd(d, p + (15383) - 3151) / 2 + rd(d, p + (15383) - 3256) / 2 + rd(d, p + (15383) - 3491) / 2 + rd(d, p + (15383) - 3555) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (-rd(d, p + (15383) - 8094) / 16 + -rd(d, p + (15383) - 7848) / 16 + -rd(d, p + (15383) - 7792) / 16 + -rd(d, p + (15383) - 7514) / 16 + -rd(d, p + (15383) - 7461) / 16 + -rd(d, p + (15383) - 7378) / 16 + -rd(d, p + (15383) - 7138) / 16 + -rd(d, p + (15383) - 7091) / 16 + rd(d, p + (15383) - 6864) / 8 + rd(d, p + (15383) - 6726) / 8 + rd(d, p + (15383) - 6656) / 8 + rd(d, p + (15383) - 6418) / 8 + rd(d, p + (15383) - 6363) / 8 + rd(d, p + (15383) - 6153) / 8 + rd(d, p + (15383) - 6017) / 8 + rd(d, p + (15383) - 5966) / 8 + -rd(d, p + (15383) - 5727) / 4 + -rd(d, p + (15383) - 5656) / 4 + -rd(d, p + (15383) - 5494) / 4 + -rd(d, p + (15383) - 5318) / 4 + -rd(d, p + (15383) - 5260) / 4 + -rd(d, p + (15383) - 5018) / 4 + -rd(d, p + (15383) - 4957) / 4 + -rd(d, p + (15383) - 4733) / 4 + rd(d, p + (15383) - 4626) / 2 + rd(d, p + (15383) - 4550) / 2 + rd(d, p + (15383) - 4338) / 2 + rd(d, p + (15383) - 4209) / 2 + rd(d, p + (15383) - 4045) / 2 + rd(d, p + (15383) - 3920) / 2 + rd(d, p + (15383) - 3799) / 2 + rd(d, p + (15383) - 3618) / 2 + rd(d, p + (1373) - 1) / 2) as i16 as i32;
    wr(d, p + (1373), acc);
    acc = (-rd(d, p + (1373) - 1) / 4 + rd(d, p + (11) - 0)) as i16 as i32;
    wr(d, p + (11), -acc);
    acc = (rd(d, p + (16373) - 989) / 2 + rd(d, p + (1370) - 1) / 2) as i16 as i32;
    wr(d, p + (1370), -acc);
    acc = (rd(d, p + (1370) - 1)) as i16 as i32;
    wr(d, p + (15383), -acc);
    (left, right)
}

fn midiverb2_effect_40(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 51) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 51) + rd(d, p + (2245) - 131) / 2) as i16 as i32;
    wr(d, p + (2245), -acc);
    acc = (-acc / 2 + rd(d, p + (2245) - 131) + rd(d, p + (2112) - 213) / 2) as i16 as i32;
    wr(d, p + (2112), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (2112) - 213) * 9 / 4 + rd(d, p + (16382) - 11316) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1897) - 62) / 2) as i16 as i32;
    wr(d, p + (1897), -acc);
    acc = (-acc / 2 + rd(d, p + (1897) - 62) + rd(d, p + (1833) - 105) / 2) as i16 as i32;
    wr(d, p + (1833), -acc);
    acc = (-acc / 2 + rd(d, p + (1833) - 105) + rd(d, p + (1726) - 189) / 2) as i16 as i32;
    wr(d, p + (1726), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (1726) - 189) * 9 / 4 + rd(d, p + (16382) - 11316) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (tmp_0 / 2) as i16 as i32;
    wr(d, p + (16382), acc);
    acc = (-rd(d, p + (16382) - 231) / 16 + -rd(d, p + (16382) - 410) / 16 + -rd(d, p + (16382) - 759) / 16 + -rd(d, p + (16382) - 1063) / 16 + rd(d, p + (16382) - 1231) / 8 + rd(d, p + (16382) - 1432) / 8 + rd(d, p + (16382) - 1740) / 8 + rd(d, p + (16382) - 1860) / 8 + rd(d, p + (16382) - 3034) / 8 + rd(d, p + (16382) - 2147) / 8 + -rd(d, p + (16382) - 2254) / 4 + -rd(d, p + (16382) - 2441) / 4 + -rd(d, p + (16382) - 1553) / 4 + -rd(d, p + (16382) - 2719) / 4 + -rd(d, p + (16382) - 2871) / 4 + -rd(d, p + (16382) - 2044) / 4 + rd(d, p + (1535) - 1) / 2) as i16 as i32;
    wr(d, p + (1535), acc);
    acc = (acc * 3 / 32 + rd(d, p + (16382) - 3119) / 32 + rd(d, p + (16382) - 3265) / 32 + rd(d, p + (16382) - 3449) / 32 + rd(d, p + (16382) - 3567) / 32 + rd(d, p + (16382) - 5032) / 32 + rd(d, p + (16382) - 3887) / 32 + rd(d, p + (16382) - 4032) / 32 + -rd(d, p + (16382) - 4725) / 16 + -rd(d, p + (16382) - 4255) / 16 + -rd(d, p + (16382) - 4490) / 16 + -rd(d, p + (16382) - 4554) / 16 + -rd(d, p + (16382) - 4847) / 16 + -rd(d, p + (16382) - 3723) / 16 + rd(d, p + (16382) - 5127) / 8 + rd(d, p + (16382) - 5233) / 8 + rd(d, p + (16382) - 5419) / 8 + rd(d, p + (16382) - 5532) / 8 + rd(d, p + (16382) - 5757) / 8 + rd(d, p + (16382) - 6857) / 8 + -rd(d, p + (16382) - 6033) / 4 + -rd(d, p + (16382) - 6157) / 4 + -rd(d, p + (16382) - 6221) / 4 + -rd(d, p + (16382) - 7124) / 4 + rd(d, p + (1532) - 1) / 2) as i16 as i32;
    wr(d, p + (1532), acc);
    left = (tmp_5) as i16;
    acc = (acc * 3 / 8 + rd(d, p + (16382) - 6532) / 8 + rd(d, p + (16382) - 6780) / 8 + rd(d, p + (16382) - 5835) / 8 + rd(d, p + (16382) - 7037) / 8 + rd(d, p + (16382) - 6444) / 8 + rd(d, p + (16382) - 7219) / 8 + rd(d, p + (16382) - 7437) / 8 + -rd(d, p + (16382) - 7734) / 4 + -rd(d, p + (16382) - 7889) / 4 + -rd(d, p + (16382) - 8032) / 4 + -rd(d, p + (16382) - 9213) / 4 + -rd(d, p + (16382) - 8445) / 4 + -rd(d, p + (16382) - 8563) / 4 + -rd(d, p + (16382) - 9511) / 4 + rd(d, p + (1529) - 1) / 2) as i16 as i32;
    wr(d, p + (1529), acc);
    right = (tmp_3) as i16;
    acc = (acc * -3 / 4 + -rd(d, p + (16382) - 8883) / 4 + -rd(d, p + (16382) - 9010) / 4 + -rd(d, p + (16382) - 9169) / 4 + -rd(d, p + (16382) - 8144) / 4 + -rd(d, p + (16382) - 9447) / 4 + -rd(d, p + (16382) - 8698) / 4 + -rd(d, p + (16382) - 9763) / 4 + -rd(d, p + (16382) - 9865) / 4 + -rd(d, p + (16382) - 10044) / 4 + -rd(d, p + (16382) - 10108) / 4 + rd(d, p + (16382) - 10283) / 2 + rd(d, p + (16382) - 10426) / 2 + rd(d, p + (16382) - 10576) / 2 + rd(d, p + (16382) - 10732) / 2 + rd(d, p + (16382) - 10813) / 2 + rd(d, p + (16382) - 11021) / 2 + rd(d, p + (16382) - 11252) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb2_effect_41(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (acc * 3 / 32 + rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 1555) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 2046) / 32 + rd(d, p + (0) - 3569) / 32 + rd(d, p + (0) - 2256) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 3121) / 32 + rd(d, p + (0) - 2721) / 32 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 3036) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 3725) / 16 + -rd(d, p + (0) - 3889) / 16 + -rd(d, p + (0) - 4034) / 16 + -rd(d, p + (0) - 4152) / 16 + -rd(d, p + (0) - 4257) / 16 + -rd(d, p + (0) - 4492) / 16 + -rd(d, p + (0) - 4556) / 16 + -rd(d, p + (0) - 4727) / 16 + -rd(d, p + (0) - 4849) / 16 + -rd(d, p + (0) - 6035) / 16 + -rd(d, p + (0) - 5129) / 16 + -rd(d, p + (0) - 5235) / 16 + -rd(d, p + (0) - 6223) / 16 + -rd(d, p + (0) - 5534) / 16 + rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5034) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6446) / 8 + rd(d, p + (0) - 6534) / 8 + rd(d, p + (0) - 6782) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 7039) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 7221) / 8 + rd(d, p + (0) - 8565) / 8 + rd(d, p + (0) - 7534) / 8 + rd(d, p + (0) - 7736) / 8 + rd(d, p + (0) - 7891) / 8 + rd(d, p + (0) - 9215) / 8 + rd(d, p + (0) - 8146) / 8 + rd(d, p + (0) - 8278) / 8 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 8700) / 4 + -rd(d, p + (0) - 8885) / 4 + -rd(d, p + (0) - 9012) / 4 + -rd(d, p + (0) - 9171) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9449) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 9765) / 4 + -rd(d, p + (0) - 9867) / 4 + -rd(d, p + (0) - 10046) / 4 + -rd(d, p + (0) - 10110) / 4 + -rd(d, p + (0) - 10285) / 4 + -rd(d, p + (0) - 12074) / 4 + -rd(d, p + (0) - 10578) / 4 + -rd(d, p + (0) - 10734) / 4 + -rd(d, p + (0) - 11715) / 4 + -rd(d, p + (0) - 11023) / 4 + -rd(d, p + (0) - 11185) / 4 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2 + rd(d, p + (0) - 13217) / 2 + rd(d, p + (0) - 13378) / 2 + rd(d, p + (0) - 13534) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb2_effect_42(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 11718)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 11718)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 2579) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 3036) / 32 + rd(d, p + (0) - 2149) / 32 + -rd(d, p + (0) - 2256) / 16 + -rd(d, p + (0) - 2443) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 2721) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 3121) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 3569) / 16 + -rd(d, p + (0) - 5034) / 16 + -rd(d, p + (0) - 3889) / 16 + -rd(d, p + (0) - 4034) / 16 + -rd(d, p + (0) - 4727) / 16 + -rd(d, p + (0) - 4257) / 16 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 4556) / 8 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4849) / 8 + rd(d, p + (0) - 3725) / 8 + rd(d, p + (0) - 5129) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 5534) / 8 + rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 6035) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 6223) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 6534) / 8 + -rd(d, p + (0) - 6782) / 4 + -rd(d, p + (0) - 5837) / 4 + -rd(d, p + (0) - 7039) / 4 + -rd(d, p + (0) - 6446) / 4 + -rd(d, p + (0) - 7221) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 7534) / 4 + -rd(d, p + (0) - 7736) / 4 + -rd(d, p + (0) - 7891) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9215) / 4 + -rd(d, p + (0) - 8278) / 4 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 8565) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 8885) / 4 + rd(d, p + (0) - 9012) / 2 + rd(d, p + (0) - 9171) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 9449) / 2 + rd(d, p + (0) - 8700) / 2 + rd(d, p + (0) - 9765) / 2 + rd(d, p + (0) - 9867) / 2 + rd(d, p + (0) - 10046) / 2 + rd(d, p + (0) - 10110) / 2 + rd(d, p + (0) - 10285) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 10578) / 2 + rd(d, p + (0) - 10734) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11023) / 2 + rd(d, p + (0) - 11185) / 2 + rd(d, p + (0) - 11254) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb2_effect_43(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 9375)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 9375)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 2721) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 1742) / 32 + rd(d, p + (0) - 1862) / 32 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 2256) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 1065) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 3036) / 16 + -rd(d, p + (0) - 4152) / 16 + -rd(d, p + (0) - 3267) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 4492) / 16 + -rd(d, p + (0) - 3725) / 16 + rd(d, p + (0) - 3889) / 8 + rd(d, p + (0) - 4034) / 8 + rd(d, p + (0) - 3121) / 8 + rd(d, p + (0) - 4257) / 8 + rd(d, p + (0) - 3569) / 8 + rd(d, p + (0) - 4556) / 8 + rd(d, p + (0) - 4727) / 8 + rd(d, p + (0) - 4849) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5129) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6159) / 8 + -rd(d, p + (0) - 5759) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 6035) / 4 + -rd(d, p + (0) - 5534) / 4 + -rd(d, p + (0) - 6223) / 4 + -rd(d, p + (0) - 6446) / 4 + -rd(d, p + (0) - 6534) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 6859) / 4 + -rd(d, p + (0) - 7039) / 4 + -rd(d, p + (0) - 7736) / 4 + -rd(d, p + (0) - 7221) / 4 + -rd(d, p + (0) - 7439) / 4 + rd(d, p + (0) - 7534) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 7891) / 2 + rd(d, p + (0) - 6782) / 2 + rd(d, p + (0) - 8146) / 2 + rd(d, p + (0) - 8278) / 2 + rd(d, p + (0) - 8447) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 8700) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_44(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 21) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 21) + rd(d, p + (2275) - 53) / 2) as i16 as i32;
    wr(d, p + (2275), -acc);
    acc = (-acc / 2 + rd(d, p + (2275) - 53) + rd(d, p + (2220) - 173) / 2) as i16 as i32;
    wr(d, p + (2220), -acc);
    acc = (-acc / 2 + rd(d, p + (2220) - 173) + rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2045) - 32) / 2) as i16 as i32;
    wr(d, p + (2045), -acc);
    acc = (-acc / 2 + rd(d, p + (2045) - 32) + rd(d, p + (2011) - 75) / 2) as i16 as i32;
    wr(d, p + (2011), -acc);
    acc = (-acc / 2 + rd(d, p + (2011) - 75) + rd(d, p + (1934) - 133) / 2) as i16 as i32;
    wr(d, p + (1934), -acc);
    acc = (-acc / 2 + rd(d, p + (1934) - 133) + rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 163) / 32 + rd(d, p + (0) - 233) / 32 + rd(d, p + (0) - 412) / 32 + rd(d, p + (0) - 534) / 32 + rd(d, p + (0) - 761) / 32 + rd(d, p + (0) - 875) / 32 + rd(d, p + (0) - 1434) / 32 + rd(d, p + (0) - 1119) / 32 + rd(d, p + (0) - 1862) / 32 + -rd(d, p + (0) - 1065) / 16 + -rd(d, p + (0) - 1555) / 16 + -rd(d, p + (0) - 1742) / 16 + -rd(d, p + (0) - 1233) / 16 + -rd(d, p + (0) - 2046) / 16 + -rd(d, p + (0) - 2149) / 16 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2443) / 16 + -rd(d, p + (0) - 3121) / 16 + -rd(d, p + (0) - 2721) / 16 + rd(d, p + (0) - 2256) / 8 + rd(d, p + (0) - 3036) / 8 + rd(d, p + (0) - 2579) / 8 + rd(d, p + (0) - 3267) / 8 + rd(d, p + (0) - 3451) / 8 + rd(d, p + (0) - 3569) / 8 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 3889) / 8 + rd(d, p + (0) - 4034) / 8 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4849) / 8 + -rd(d, p + (0) - 3725) / 4 + -rd(d, p + (0) - 4556) / 4 + -rd(d, p + (0) - 4727) / 4 + -rd(d, p + (0) - 4257) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 5837) / 4 + -rd(d, p + (0) - 5235) / 4 + -rd(d, p + (0) - 5421) / 4 + -rd(d, p + (0) - 6159) / 4 + rd(d, p + (0) - 5759) / 2 + rd(d, p + (0) - 5129) / 2 + rd(d, p + (0) - 6035) / 2 + rd(d, p + (0) - 5534) / 2 + rd(d, p + (0) - 6223) / 2 + rd(d, p + (0) - 6446) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_45(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (2190) - 1)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (2187) - 1)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (2193) - 1) / 2 + rd(d, p + (0) - 1) / 2) as i16 as i32;
    wr(d, p + (2193), acc);
    acc = (-rd(d, p + (11) - 1) / 4 + rd(d, p + (2184) - 1) / 2) as i16 as i32;
    wr(d, p + (2184), acc);
    acc = (acc * 3 / 2 + rd(d, p + (2193) - 1) + rd(d, p + (16381) - 13) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 13) + rd(d, p + (16366) - 83) / 2) as i16 as i32;
    wr(d, p + (16366), -acc);
    acc = (-acc / 2 + rd(d, p + (16366) - 83) + rd(d, p + (16281) - 116) / 2) as i16 as i32;
    wr(d, p + (16281), -acc);
    acc = (-acc / 2 + rd(d, p + (16281) - 116) + rd(d, p + (16163) - 239) / 2) as i16 as i32;
    wr(d, p + (16163), -acc);
    acc = (-acc / 2 + rd(d, p + (16163) - 239) + rd(d, p + (15922) - 339) / 2) as i16 as i32;
    wr(d, p + (15922), -acc);
    acc = (-acc / 2 + rd(d, p + (15922) - 339) + rd(d, p + (15581) - 481) / 2) as i16 as i32;
    wr(d, p + (15581), -acc);
    acc = (-acc / 2 + rd(d, p + (15581) - 481) + rd(d, p + (15098) - 555) / 2) as i16 as i32;
    wr(d, p + (15098), -acc);
    acc = (-acc / 2 + rd(d, p + (15098) - 555) + rd(d, p + (14541) - 823) / 2) as i16 as i32;
    wr(d, p + (14541), -acc);
    acc = (-acc / 2 + rd(d, p + (14541) - 823) + rd(d, p + (13716) - 999) / 2) as i16 as i32;
    wr(d, p + (13716), -acc);
    acc = (-acc / 2 + rd(d, p + (13716) - 999) + rd(d, p + (12715) - 1100) / 2) as i16 as i32;
    wr(d, p + (12715), -acc);
    acc = (-acc / 2 + rd(d, p + (12715) - 1100) + rd(d, p + (11613) - 1347) / 2) as i16 as i32;
    wr(d, p + (11613), -acc);
    acc = (-acc / 2 + rd(d, p + (11613) - 1347) + rd(d, p + (10264) - 1563) / 2) as i16 as i32;
    wr(d, p + (10264), -acc);
    acc = (-acc / 2 + rd(d, p + (10264) - 1563) + rd(d, p + (8699) - 1841) / 2) as i16 as i32;
    wr(d, p + (8699), -acc);
    acc = (-acc / 2 + rd(d, p + (8699) - 1841) + rd(d, p + (6856) - 2001) / 2) as i16 as i32;
    wr(d, p + (6856), -acc);
    acc = (-acc / 2 + rd(d, p + (6856) - 2001) + rd(d, p + (4853) - 2083) / 2) as i16 as i32;
    wr(d, p + (4853), -acc);
    acc = (-acc / 2 + rd(d, p + (4853) - 2083)) as i16 as i32;
    wr(d, p + (11), -acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (2666) - 147) / 2) as i16 as i32;
    wr(d, p + (2666), -acc);
    acc = (-acc / 2 + rd(d, p + (2666) - 147) + rd(d, p + (2768) - 43) / 2) as i16 as i32;
    wr(d, p + (2768), -acc);
    acc = (-acc / 2 + rd(d, p + (2768) - 43) + rd(d, p + (2723) - 55) / 2) as i16 as i32;
    wr(d, p + (2723), -acc);
    acc = (-acc / 2 + rd(d, p + (2723) - 55)) as i16 as i32;
    wr(d, p + (2190), -acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (2444) - 249) / 2) as i16 as i32;
    wr(d, p + (2444), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (2444) - 249) + rd(d, p + (2494) - 48) / 2) as i16 as i32;
    wr(d, p + (2494), -acc);
    acc = (-acc / 2 + rd(d, p + (2494) - 48) + rd(d, p + (2517) - 21) / 2) as i16 as i32;
    wr(d, p + (2517), -acc);
    acc = (-acc / 2 + rd(d, p + (2517) - 21)) as i16 as i32;
    wr(d, p + (2187), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_46(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 0)) as i16 as i32;
    wr(d, p + (3166), -acc);
    acc = (rd(d, p + (3166) - 2000)) as i16 as i32;
    wr(d, p + (0), -acc);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (4298) - 51) / 2) as i16 as i32;
    wr(d, p + (4298), -acc);
    acc = (-acc / 2 + rd(d, p + (4298) - 51) + rd(d, p + (4245) - 231) / 2) as i16 as i32;
    wr(d, p + (4245), -acc);
    acc = (-acc / 2 + rd(d, p + (4245) - 231) + rd(d, p + (4012) - 313) / 2) as i16 as i32;
    wr(d, p + (4012), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (4012) - 313) * 9 / 4 + rd(d, p + (16373) - 11307) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (3697) - 62) / 2) as i16 as i32;
    wr(d, p + (3697), -acc);
    acc = (-acc / 2 + rd(d, p + (3697) - 62) + rd(d, p + (3633) - 165) / 2) as i16 as i32;
    wr(d, p + (3633), -acc);
    acc = (-acc / 2 + rd(d, p + (3633) - 165) + rd(d, p + (3466) - 289) / 2) as i16 as i32;
    wr(d, p + (3466), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (3466) - 289) * 9 / 4 + rd(d, p + (16373) - 11307) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 10) / 2) as i16 as i32;
    wr(d, p + (16373), acc);
    acc = (acc * -3 / 16 + -rd(d, p + (16373) - 401) / 16 + -rd(d, p + (16373) - 750) / 16 + -rd(d, p + (16373) - 1054) / 16 + rd(d, p + (16373) - 1222) / 8 + rd(d, p + (16373) - 1423) / 8 + rd(d, p + (16373) - 1731) / 8 + rd(d, p + (16373) - 1851) / 8 + rd(d, p + (16373) - 2138) / 8 + -rd(d, p + (16373) - 2245) / 4 + -rd(d, p + (16373) - 2432) / 4 + -rd(d, p + (16373) - 1544) / 4 + -rd(d, p + (16373) - 2710) / 4 + -rd(d, p + (16373) - 2862) / 4 + -rd(d, p + (16373) - 2035) / 4 + rd(d, p + (3175) - 1) / 2) as i16 as i32;
    wr(d, p + (3175), acc);
    acc = (acc * 3 / 32 + rd(d, p + (16373) - 3110) / 32 + rd(d, p + (16373) - 3256) / 32 + rd(d, p + (16373) - 3558) / 32 + rd(d, p + (16373) - 5023) / 32 + rd(d, p + (16373) - 3878) / 32 + rd(d, p + (16373) - 4023) / 32 + -rd(d, p + (16373) - 4716) / 16 + -rd(d, p + (16373) - 4246) / 16 + -rd(d, p + (16373) - 4481) / 16 + -rd(d, p + (16373) - 4838) / 16 + -rd(d, p + (16373) - 3714) / 16 + rd(d, p + (16373) - 5118) / 8 + rd(d, p + (16373) - 5224) / 8 + rd(d, p + (16373) - 5410) / 8 + rd(d, p + (16373) - 5523) / 8 + rd(d, p + (16373) - 5748) / 8 + rd(d, p + (16373) - 6848) / 8 + -rd(d, p + (16373) - 6024) / 4 + -rd(d, p + (16373) - 6212) / 4 + -rd(d, p + (16373) - 7115) / 4 + rd(d, p + (3172) - 1) / 2) as i16 as i32;
    wr(d, p + (3172), acc);
    left = (tmp_5) as i16;
    acc = (acc * 3 / 8 + rd(d, p + (16373) - 6523) / 8 + rd(d, p + (16373) - 5826) / 8 + rd(d, p + (16373) - 7028) / 8 + rd(d, p + (16373) - 6435) / 8 + rd(d, p + (16373) - 7210) / 8 + rd(d, p + (16373) - 7428) / 8 + -rd(d, p + (16373) - 7725) / 4 + -rd(d, p + (16373) - 7880) / 4 + -rd(d, p + (16373) - 8023) / 4 + -rd(d, p + (16373) - 9204) / 4 + -rd(d, p + (16373) - 8436) / 4 + -rd(d, p + (16373) - 8554) / 4 + -rd(d, p + (16373) - 9502) / 4 + rd(d, p + (3169) - 1) / 2) as i16 as i32;
    wr(d, p + (3169), acc);
    right = (tmp_3) as i16;
    acc = (acc * -3 / 4 + -rd(d, p + (16373) - 8874) / 4 + -rd(d, p + (16373) - 9001) / 4 + -rd(d, p + (16373) - 9160) / 4 + -rd(d, p + (16373) - 8135) / 4 + -rd(d, p + (16373) - 9438) / 4 + -rd(d, p + (16373) - 8689) / 4 + -rd(d, p + (16373) - 9754) / 4 + -rd(d, p + (16373) - 9856) / 4 + -rd(d, p + (16373) - 10035) / 4 + -rd(d, p + (16373) - 10099) / 4 + rd(d, p + (16373) - 10274) / 2 + rd(d, p + (16373) - 10417) / 2 + rd(d, p + (16373) - 10567) / 2 + rd(d, p + (16373) - 10723) / 2 + rd(d, p + (16373) - 10804) / 2 + rd(d, p + (16373) - 11012) / 2 + rd(d, p + (16373) - 11243) / 2) as i16 as i32;
    wr(d, p + (11), -acc);
    (left, right)
}

fn midiverb2_effect_47(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 51) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 51) + rd(d, p + (2245) - 131) / 2) as i16 as i32;
    wr(d, p + (2245), -acc);
    acc = (-acc / 2 + rd(d, p + (2245) - 131) + rd(d, p + (2112) - 213) / 2) as i16 as i32;
    wr(d, p + (2112), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (2112) - 213) * 9 / 4 + rd(d, p + (16373) - 11307) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1897) - 62) / 2) as i16 as i32;
    wr(d, p + (1897), -acc);
    acc = (-acc / 2 + rd(d, p + (1897) - 62) + rd(d, p + (1833) - 105) / 2) as i16 as i32;
    wr(d, p + (1833), -acc);
    acc = (-acc / 2 + rd(d, p + (1833) - 105) + rd(d, p + (1726) - 189) / 2) as i16 as i32;
    wr(d, p + (1726), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (1726) - 189) * 9 / 4 + rd(d, p + (16373) - 11307) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (tmp_0 / 2 + rd(d, p + (16373) - 11307) / 2) as i16 as i32;
    wr(d, p + (16383), -acc);
    acc = (rd(d, p + (16383) - 9) / 2) as i16 as i32;
    wr(d, p + (16373), acc);
    acc = (rd(d, p + (16373) - 1222) / 8 + rd(d, p + (16373) - 1423) / 8 + rd(d, p + (16373) - 1731) / 8 + rd(d, p + (16373) - 1851) / 8 + rd(d, p + (16373) - 3025) / 8 + rd(d, p + (16373) - 2138) / 8 + -rd(d, p + (16373) - 2245) / 4 + -rd(d, p + (16373) - 2432) / 4 + -rd(d, p + (16373) - 1544) / 4 + -rd(d, p + (16373) - 2710) / 4 + -rd(d, p + (16373) - 2862) / 4 + -rd(d, p + (16373) - 2035) / 4 + rd(d, p + (1535) - 1) / 2) as i16 as i32;
    wr(d, p + (1535), acc);
    acc = (acc * 3 / 32 + rd(d, p + (16373) - 3110) / 32 + rd(d, p + (16373) - 3256) / 32 + rd(d, p + (16373) - 3440) / 32 + rd(d, p + (16373) - 3558) / 32 + rd(d, p + (16373) - 5023) / 32 + rd(d, p + (16373) - 3878) / 32 + rd(d, p + (16373) - 4023) / 32 + -rd(d, p + (16373) - 4716) / 16 + -rd(d, p + (16373) - 4246) / 16 + -rd(d, p + (16373) - 4481) / 16 + -rd(d, p + (16373) - 4545) / 16 + -rd(d, p + (16373) - 4838) / 16 + -rd(d, p + (16373) - 3714) / 16 + rd(d, p + (16373) - 5118) / 8 + rd(d, p + (16373) - 5224) / 8 + rd(d, p + (16373) - 5410) / 8 + rd(d, p + (16373) - 5523) / 8 + rd(d, p + (16373) - 5748) / 8 + rd(d, p + (16373) - 6848) / 8 + -rd(d, p + (16373) - 6024) / 4 + -rd(d, p + (16373) - 6148) / 4 + -rd(d, p + (16373) - 6212) / 4 + -rd(d, p + (16373) - 7115) / 4 + rd(d, p + (1532) - 1) / 2) as i16 as i32;
    wr(d, p + (1532), acc);
    left = (tmp_5) as i16;
    acc = (acc * 3 / 8 + rd(d, p + (16373) - 6523) / 8 + rd(d, p + (16373) - 6771) / 8 + rd(d, p + (16373) - 5826) / 8 + rd(d, p + (16373) - 7028) / 8 + rd(d, p + (16373) - 6435) / 8 + rd(d, p + (16373) - 7210) / 8 + rd(d, p + (16373) - 7428) / 8 + -rd(d, p + (16373) - 7725) / 4 + -rd(d, p + (16373) - 7880) / 4 + -rd(d, p + (16373) - 8023) / 4 + -rd(d, p + (16373) - 9204) / 4 + -rd(d, p + (16373) - 8436) / 4 + -rd(d, p + (16373) - 8554) / 4 + -rd(d, p + (16373) - 9502) / 4 + rd(d, p + (1529) - 1) / 2) as i16 as i32;
    wr(d, p + (1529), acc);
    right = (tmp_3) as i16;
    acc = (acc * -3 / 4 + -rd(d, p + (16373) - 8874) / 4 + -rd(d, p + (16373) - 9001) / 4 + -rd(d, p + (16373) - 9160) / 4 + -rd(d, p + (16373) - 8135) / 4 + -rd(d, p + (16373) - 9438) / 4 + -rd(d, p + (16373) - 8689) / 4 + -rd(d, p + (16373) - 9754) / 4 + -rd(d, p + (16373) - 9856) / 4 + -rd(d, p + (16373) - 10035) / 4 + -rd(d, p + (16373) - 10099) / 4 + rd(d, p + (16373) - 10274) / 2 + rd(d, p + (16373) - 10417) / 2 + rd(d, p + (16373) - 10567) / 2 + rd(d, p + (16373) - 10723) / 2 + rd(d, p + (16373) - 10804) / 2 + rd(d, p + (16373) - 11012) / 2 + rd(d, p + (16373) - 11243) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb2_effect_48(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (2298) - 51) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 51) + rd(d, p + (2245) - 131) / 2) as i16 as i32;
    wr(d, p + (2245), -acc);
    acc = (-acc / 2 + rd(d, p + (2245) - 131) + rd(d, p + (2112) - 213) / 2) as i16 as i32;
    wr(d, p + (2112), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (2112) - 213) * 9 / 4 + rd(d, p + (16364) - 11298) * 9 / 4) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (11) - 2) / 2 + rd(d, p + (1897) - 62) / 2) as i16 as i32;
    wr(d, p + (1897), -acc);
    acc = (-acc / 2 + rd(d, p + (1897) - 62) + rd(d, p + (1833) - 105) / 2) as i16 as i32;
    wr(d, p + (1833), -acc);
    acc = (-acc / 2 + rd(d, p + (1833) - 105) + rd(d, p + (1726) - 189) / 2) as i16 as i32;
    wr(d, p + (1726), -acc);
    acc = (acc * -9 / 8 + rd(d, p + (1726) - 189) * 9 / 4 + rd(d, p + (16364) - 11298) * 9 / 4) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 3)) as i16 as i32;
    wr(d, p + (1526), -acc);
    acc = (rd(d, p + (1526) - 1300)) as i16 as i32;
    wr(d, p + (16380), -acc);
    acc = (rd(d, p + (16364) - 11298) * 3 / 4 + rd(d, p + (0) - 0) / 2) as i16 as i32;
    wr(d, p + (16364), -acc);
    acc = (rd(d, p + (16364) - 1213) / 8 + rd(d, p + (16364) - 2129) / 8 + -rd(d, p + (16364) - 2236) / 4 + -rd(d, p + (16364) - 2853) / 4 + -rd(d, p + (16364) - 2026) / 4 + rd(d, p + (1535) - 1) / 2) as i16 as i32;
    wr(d, p + (1535), acc);
    acc = (acc * 3 / 32 + rd(d, p + (16364) - 3101) / 32 + rd(d, p + (16364) - 3549) / 32 + rd(d, p + (16364) - 5014) / 32 + rd(d, p + (16364) - 3869) / 32 + rd(d, p + (16364) - 4014) / 32 + -rd(d, p + (16364) - 4707) / 16 + -rd(d, p + (16364) - 4237) / 16 + -rd(d, p + (16364) - 4472) / 16 + -rd(d, p + (16364) - 4536) / 16 + -rd(d, p + (16364) - 4829) / 16 + -rd(d, p + (16364) - 3705) / 16 + rd(d, p + (16364) - 5109) / 8 + rd(d, p + (16364) - 5215) / 8 + rd(d, p + (16364) - 5401) / 8 + rd(d, p + (16364) - 5514) / 8 + rd(d, p + (16364) - 5739) / 8 + rd(d, p + (16364) - 6839) / 8 + -rd(d, p + (16364) - 6015) / 4 + -rd(d, p + (16364) - 6139) / 4 + -rd(d, p + (16364) - 6203) / 4 + -rd(d, p + (16364) - 7106) / 4 + rd(d, p + (1532) - 1) / 2) as i16 as i32;
    wr(d, p + (1532), acc);
    left = (tmp_5) as i16;
    acc = (acc * 3 / 8 + rd(d, p + (16364) - 6514) / 8 + rd(d, p + (16364) - 6762) / 8 + rd(d, p + (16364) - 5817) / 8 + rd(d, p + (16364) - 7019) / 8 + rd(d, p + (16364) - 6426) / 8 + rd(d, p + (16364) - 7201) / 8 + rd(d, p + (16364) - 7419) / 8 + -rd(d, p + (16364) - 7716) / 4 + -rd(d, p + (16364) - 7871) / 4 + -rd(d, p + (16364) - 8014) / 4 + -rd(d, p + (16364) - 9195) / 4 + -rd(d, p + (16364) - 8427) / 4 + -rd(d, p + (16364) - 8545) / 4 + -rd(d, p + (16364) - 9493) / 4 + rd(d, p + (1529) - 1) / 2) as i16 as i32;
    wr(d, p + (1529), acc);
    right = (tmp_3) as i16;
    acc = (acc * -3 / 4 + -rd(d, p + (16364) - 8865) / 4 + -rd(d, p + (16364) - 8992) / 4 + -rd(d, p + (16364) - 9151) / 4 + -rd(d, p + (16364) - 8126) / 4 + -rd(d, p + (16364) - 9429) / 4 + -rd(d, p + (16364) - 8680) / 4 + -rd(d, p + (16364) - 9745) / 4 + -rd(d, p + (16364) - 9847) / 4 + -rd(d, p + (16364) - 10026) / 4 + -rd(d, p + (16364) - 10090) / 4 + rd(d, p + (16364) - 10265) / 2 + rd(d, p + (16364) - 10408) / 2 + rd(d, p + (16364) - 10558) / 2 + rd(d, p + (16364) - 10714) / 2 + rd(d, p + (16364) - 10795) / 2 + rd(d, p + (16364) - 11003) / 2 + rd(d, p + (16364) - 11234) / 2) as i16 as i32;
    wr(d, p + (11), acc);
    (left, right)
}

fn midiverb2_effect_49(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (191) - 1)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (188) - 1)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (194) - 1) / 2 + rd(d, p + (0) - 2000) / 2) as i16 as i32;
    wr(d, p + (194), acc);
    acc = (-rd(d, p + (11) - 1) / 4 + rd(d, p + (185) - 1) / 2) as i16 as i32;
    wr(d, p + (185), acc);
    acc = (acc * 3 / 2 + rd(d, p + (194) - 1) + rd(d, p + (14382) - 13) / 2) as i16 as i32;
    wr(d, p + (14382), -acc);
    acc = (-acc / 2 + rd(d, p + (14382) - 13) + rd(d, p + (14367) - 83) / 2) as i16 as i32;
    wr(d, p + (14367), -acc);
    acc = (-acc / 2 + rd(d, p + (14367) - 83) + rd(d, p + (14282) - 116) / 2) as i16 as i32;
    wr(d, p + (14282), -acc);
    acc = (-acc / 2 + rd(d, p + (14282) - 116) + rd(d, p + (14164) - 239) / 2) as i16 as i32;
    wr(d, p + (14164), -acc);
    acc = (-acc / 2 + rd(d, p + (14164) - 239) + rd(d, p + (13923) - 339) / 2) as i16 as i32;
    wr(d, p + (13923), -acc);
    acc = (-acc / 2 + rd(d, p + (13923) - 339) + rd(d, p + (13582) - 481) / 2) as i16 as i32;
    wr(d, p + (13582), -acc);
    acc = (-acc / 2 + rd(d, p + (13582) - 481) + rd(d, p + (13099) - 555) / 2) as i16 as i32;
    wr(d, p + (13099), -acc);
    acc = (-acc / 2 + rd(d, p + (13099) - 555) + rd(d, p + (12542) - 823) / 2) as i16 as i32;
    wr(d, p + (12542), -acc);
    acc = (-acc / 2 + rd(d, p + (12542) - 823) + rd(d, p + (11717) - 999) / 2) as i16 as i32;
    wr(d, p + (11717), -acc);
    acc = (-acc / 2 + rd(d, p + (11717) - 999) + rd(d, p + (10716) - 1100) / 2) as i16 as i32;
    wr(d, p + (10716), -acc);
    acc = (-acc / 2 + rd(d, p + (10716) - 1100) + rd(d, p + (9614) - 1347) / 2) as i16 as i32;
    wr(d, p + (9614), -acc);
    acc = (-acc / 2 + rd(d, p + (9614) - 1347) + rd(d, p + (8265) - 1563) / 2) as i16 as i32;
    wr(d, p + (8265), -acc);
    acc = (-acc / 2 + rd(d, p + (8265) - 1563) + rd(d, p + (6700) - 1841) / 2) as i16 as i32;
    wr(d, p + (6700), -acc);
    acc = (-acc / 2 + rd(d, p + (6700) - 1841) + rd(d, p + (4857) - 2001) / 2) as i16 as i32;
    wr(d, p + (4857), -acc);
    acc = (-acc / 2 + rd(d, p + (4857) - 2001) + rd(d, p + (2854) - 2083) / 2) as i16 as i32;
    wr(d, p + (2854), -acc);
    acc = (-acc / 2 + rd(d, p + (2854) - 2083)) as i16 as i32;
    wr(d, p + (11), -acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (667) - 147) / 2) as i16 as i32;
    wr(d, p + (667), -acc);
    acc = (-acc / 2 + rd(d, p + (667) - 147) + rd(d, p + (769) - 43) / 2) as i16 as i32;
    wr(d, p + (769), -acc);
    acc = (-acc / 2 + rd(d, p + (769) - 43) + rd(d, p + (724) - 55) / 2) as i16 as i32;
    wr(d, p + (724), -acc);
    acc = (-acc / 2 + rd(d, p + (724) - 55)) as i16 as i32;
    wr(d, p + (191), -acc);
    acc = (rd(d, p + (11) - 0) + rd(d, p + (445) - 249) / 2) as i16 as i32;
    wr(d, p + (445), -acc);
    left = (tmp_5) as i16;
    acc = (-acc / 2 + rd(d, p + (445) - 249) + rd(d, p + (495) - 48) / 2) as i16 as i32;
    wr(d, p + (495), -acc);
    acc = (-acc / 2 + rd(d, p + (495) - 48) + rd(d, p + (518) - 21) / 2) as i16 as i32;
    wr(d, p + (518), -acc);
    acc = (-acc / 2 + rd(d, p + (518) - 21)) as i16 as i32;
    wr(d, p + (188), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_50(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (-acc / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_51(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_52(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (-acc / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_53(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_54(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (-acc / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_55(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_56(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (-acc / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_57(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_58(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (-acc / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_59(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_60(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_61(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_62(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_63(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_64(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_65(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_66(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_67(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_68(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_69(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_0 = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    let mut tmp_fa7 = 0i32;
    let mut tmp_fa8 = 0i32;
    let mut tmp_fa9 = 0i32;
    let mut tmp_faa = 0i32;
    let mut tmp_fab = 0i32;
    let mut tmp_fac = 0i32;
    let mut tmp_fad = 0i32;
    let mut tmp_1f47 = 0i32;
    let mut tmp_1f48 = 0i32;
    let mut tmp_1f49 = 0i32;
    let mut tmp_1f4a = 0i32;
    let mut tmp_1f4b = 0i32;
    let mut tmp_1f4c = 0i32;
    let mut tmp_1f4d = 0i32;
    tmp_0 = (input) as i16 as i32;
    acc = (tmp_0) as i16 as i32;
    wr(d, p + (16282), acc);
    acc = (rd(d, p + ((16374 - (lfo1 >> 8))) - 88) * 27 / 1024 + rd(d, p + ((16282 - (lfo1 >> 8))) - 5) * 27 / 1024 + rd(d, p + ((16374 - (lfo1 >> 8))) - 89) * -27 / 512 + rd(d, p + ((16282 - (lfo1 >> 8))) - 4) * -27 / 512 + rd(d, p + ((16374 - (lfo1 >> 8))) - 90) * 27 / 256 + rd(d, p + ((16282 - (lfo1 >> 8))) - 3) * 27 / 256 + rd(d, p + ((16374 - (lfo1 >> 8))) - 91) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 2) * -27 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 0) * 81 / 128 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) * 81 / 128) as i16 as i32;
    tmp_1f4a = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f48 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4c = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f48 / 2) as i16 as i32;
    tmp_1f47 = (acc) as i16 as i32;
    acc = (tmp_1f48 / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f49 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2 + tmp_1f4c / 2) as i16 as i32;
    tmp_1f4d = (acc) as i16 as i32;
    acc = (tmp_1f4c / 2 + tmp_1f4a / 2) as i16 as i32;
    tmp_1f4b = (acc) as i16 as i32;
    match (lfo1 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo1 >> 8))) - 0) / 2 + tmp_1f47 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_1f47) as i16 as i32;
        }
        3 => {
    acc = (tmp_1f47 / 2 + tmp_1f48 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_1f48) as i16 as i32;
        }
        5 => {
    acc = (tmp_1f48 / 2 + tmp_1f49 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_1f49) as i16 as i32;
        }
        7 => {
    acc = (tmp_1f49 / 2 + tmp_1f4a / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_1f4a) as i16 as i32;
        }
        9 => {
    acc = (tmp_1f4a / 2 + tmp_1f4b / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_1f4b) as i16 as i32;
        }
        11 => {
    acc = (tmp_1f4b / 2 + tmp_1f4c / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_1f4c) as i16 as i32;
        }
        13 => {
    acc = (tmp_1f4c / 2 + tmp_1f4d / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_1f4d) as i16 as i32;
        }
        15 => {
    acc = (tmp_1f4d / 2 + rd(d, p + ((16282 - (lfo1 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16374 - (lfo2 >> 8))) - 88) * 9 / 512 + rd(d, p + ((16282 - (lfo2 >> 8))) - 5) * 9 / 512 + rd(d, p + ((16374 - (lfo2 >> 8))) - 89) * -9 / 256 + rd(d, p + ((16282 - (lfo2 >> 8))) - 4) * -9 / 256 + rd(d, p + ((16374 - (lfo2 >> 8))) - 90) * 9 / 128 + rd(d, p + ((16282 - (lfo2 >> 8))) - 3) * 9 / 128 + rd(d, p + ((16374 - (lfo2 >> 8))) - 91) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 2) * -9 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 0) * 27 / 64 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) * 27 / 64) as i16 as i32;
    wr(d, p + ((4006 - (lfo2 >> 8))), acc);
    acc = (acc * 3 / 2) as i16 as i32;
    tmp_faa = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa8 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fac = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa8 / 2) as i16 as i32;
    tmp_fa7 = (acc) as i16 as i32;
    acc = (tmp_fa8 / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fa9 = (acc) as i16 as i32;
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2 + tmp_fac / 2) as i16 as i32;
    tmp_fad = (acc) as i16 as i32;
    acc = (tmp_fac / 2 + tmp_faa / 2) as i16 as i32;
    tmp_fab = (acc) as i16 as i32;
    match (lfo2 >> 4) & 15 {
        0 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0)) as i16 as i32;
        }
        1 => {
    acc = (rd(d, p + ((16282 - (lfo2 >> 8))) - 0) / 2 + tmp_fa7 / 2) as i16 as i32;
        }
        2 => {
    acc = (tmp_fa7) as i16 as i32;
        }
        3 => {
    acc = (tmp_fa7 / 2 + tmp_fa8 / 2) as i16 as i32;
        }
        4 => {
    acc = (tmp_fa8) as i16 as i32;
        }
        5 => {
    acc = (tmp_fa8 / 2 + tmp_fa9 / 2) as i16 as i32;
        }
        6 => {
    acc = (tmp_fa9) as i16 as i32;
        }
        7 => {
    acc = (tmp_fa9 / 2 + tmp_faa / 2) as i16 as i32;
        }
        8 => {
    acc = (tmp_faa) as i16 as i32;
        }
        9 => {
    acc = (tmp_faa / 2 + tmp_fab / 2) as i16 as i32;
        }
        10 => {
    acc = (tmp_fab) as i16 as i32;
        }
        11 => {
    acc = (tmp_fab / 2 + tmp_fac / 2) as i16 as i32;
        }
        12 => {
    acc = (tmp_fac) as i16 as i32;
        }
        13 => {
    acc = (tmp_fac / 2 + tmp_fad / 2) as i16 as i32;
        }
        14 => {
    acc = (tmp_fad) as i16 as i32;
        }
        15 => {
    acc = (tmp_fad / 2 + rd(d, p + ((16282 - (lfo2 >> 8))) - 1) / 2) as i16 as i32;
        }
        _ => {}
    }
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    acc = (acc * 3 / 32768) as i16 as i32;
    wr(d, p + (16374), -acc);
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_70(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1172)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1172)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_71(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1758)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1758)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_72(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 2344)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2344)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_73(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 3516)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 3516)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_74(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 4688)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 4688)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_75(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 4922)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 4922)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_76(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5156)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 5156)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_77(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5391)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 5391)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_78(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5625)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 5625)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_79(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5859)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 5859)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_80(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 6094)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 6094)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_81(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 6328)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 6328)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_82(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 6563)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 6563)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_83(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 6797)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 6797)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_84(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 7031)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_85(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7500)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 7500)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_86(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 7969)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 7969)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_87(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 9141)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 9141)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_88(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 11719)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 11719)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_89(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 14063)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 14063)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_90(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 15705) / 2 + rd(d, p + (0) - 7853) + rd(d, p + (330) - 41) / 2) as i16 as i32;
    wr(d, p + (330), -acc);
    acc = (-acc / 2 + rd(d, p + (330) - 41) + rd(d, p + (287) - 195) / 2) as i16 as i32;
    wr(d, p + (287), -acc);
    acc = (-acc / 2 + rd(d, p + (287) - 195) + rd(d, p + (90) - 63) / 2) as i16 as i32;
    wr(d, p + (90), -acc);
    acc = (-acc / 2 + rd(d, p + (90) - 63)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 15705) / 2 + rd(d, p + (0) - 7853) + rd(d, p + (677) - 49) / 2) as i16 as i32;
    wr(d, p + (677), -acc);
    acc = (-acc / 2 + rd(d, p + (677) - 49) + rd(d, p + (626) - 114) / 2) as i16 as i32;
    wr(d, p + (626), -acc);
    acc = (-acc / 2 + rd(d, p + (626) - 114) + rd(d, p + (510) - 178) / 2) as i16 as i32;
    wr(d, p + (510), -acc);
    acc = (-acc / 2 + rd(d, p + (510) - 178)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_91(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 5460) + rd(d, p + (0) - 16378) / 2) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    acc = (tmp_5 * 21 / 32) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 10919) + rd(d, p + (0) - 16378) / 2) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (tmp_3 * 21 / 32) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_92(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 13998) / 128 + rd(d, p + (0) - 13968) / 128 + rd(d, p + (0) - 13955) / 128 + rd(d, p + (0) - 13957) / 128 + rd(d, p + (0) - 13815) / 128 + -rd(d, p + (0) - 13819) / 64 + -rd(d, p + (0) - 13749) / 64 + -rd(d, p + (0) - 13756) / 64 + -rd(d, p + (0) - 13676) / 64 + rd(d, p + (0) - 13552) / 32 + rd(d, p + (0) - 13319) / 32 + rd(d, p + (0) - 13185) / 32 + rd(d, p + (0) - 12927) / 32 + rd(d, p + (0) - 12656) / 32 + -rd(d, p + (0) - 12338) / 16 + -rd(d, p + (0) - 11919) / 16 + -rd(d, p + (0) - 11361) / 16 + -rd(d, p + (0) - 10683) / 16 + rd(d, p + (0) - 9835) / 8 + rd(d, p + (0) - 8775) / 8 + -rd(d, p + (0) - 9450) / 4 + -rd(d, p + (0) - 5194) / 4 + rd(d, p + (0) - 4501) / 2 + rd(d, p + (0) - 2724) / 2 + rd(d, p + (0) - 781) / 2) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 13986) / 128 + rd(d, p + (0) - 13919) / 128 + rd(d, p + (0) - 13918) / 128 + rd(d, p + (0) - 13972) / 128 + rd(d, p + (0) - 13807) / 128 + rd(d, p + (0) - 13854) / 128 + -rd(d, p + (0) - 13727) / 64 + -rd(d, p + (0) - 13619) / 64 + -rd(d, p + (0) - 13519) / 64 + rd(d, p + (0) - 13457) / 32 + rd(d, p + (0) - 13289) / 32 + rd(d, p + (0) - 13071) / 32 + -rd(d, p + (0) - 12727) / 16 + -rd(d, p + (0) - 12449) / 16 + -rd(d, p + (0) - 12016) / 16 + -rd(d, p + (0) - 11571) / 16 + rd(d, p + (0) - 10981) / 8 + rd(d, p + (0) - 10160) / 8 + rd(d, p + (0) - 9145) / 8 + rd(d, p + (0) - 8913) / 8 + -rd(d, p + (0) - 6873) / 4 + -rd(d, p + (0) - 4048) / 4 + rd(d, p + (0) - 3041) / 2 + rd(d, p + (0) - 1121) / 2) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_93(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (12) - 1) / 2 + rd(d, p + (2298) - 67) / 2) as i16 as i32;
    wr(d, p + (2298), -acc);
    acc = (-acc / 2 + rd(d, p + (2298) - 67) + rd(d, p + (2229) - 121) / 2) as i16 as i32;
    wr(d, p + (2229), -acc);
    acc = (-acc / 2 + rd(d, p + (2229) - 121) + rd(d, p + (0) - 14000)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (10) - 1) / 2 + rd(d, p + (1931) - 45) / 2) as i16 as i32;
    wr(d, p + (1931), -acc);
    acc = (-acc / 2 + rd(d, p + (1931) - 45) + rd(d, p + (1884) - 98) / 2) as i16 as i32;
    wr(d, p + (1884), -acc);
    acc = (-acc / 2 + rd(d, p + (1884) - 98)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 1065) / 32 + rd(d, p + (0) - 1233) / 32 + rd(d, p + (0) - 1555) / 32 + rd(d, p + (0) - 1862) / 32 + rd(d, p + (0) - 3569) / 32 + rd(d, p + (0) - 2443) / 32 + rd(d, p + (0) - 2721) / 32 + -rd(d, p + (0) - 2873) / 16 + -rd(d, p + (0) - 2579) / 16 + -rd(d, p + (0) - 3451) / 16 + -rd(d, p + (0) - 13112) / 16 + -rd(d, p + (0) - 3725) / 16 + -rd(d, p + (0) - 3889) / 16 + rd(d, p + (0) - 4152) / 8 + rd(d, p + (0) - 4492) / 8 + rd(d, p + (0) - 6035) / 8 + rd(d, p + (0) - 12345) / 8 + rd(d, p + (0) - 11111) / 8 + rd(d, p + (0) - 5235) / 8 + rd(d, p + (0) - 5534) / 8 + -rd(d, p + (0) - 5759) / 4 + -rd(d, p + (0) - 5034) / 4 + -rd(d, p + (0) - 10211) / 4 + -rd(d, p + (0) - 9123) / 4 + -rd(d, p + (0) - 12781) / 4 + -rd(d, p + (0) - 5421) / 4 + -rd(d, p + (0) - 6534) / 4 + rd(d, p + (0) - 6859) / 2 + rd(d, p + (0) - 7126) / 2 + rd(d, p + (0) - 8565) / 2 + rd(d, p + (0) - 7736) / 2 + rd(d, p + (0) - 9215) / 2 + rd(d, p + (0) - 8146) / 2) as i16 as i32;
    wr(d, p + (10), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    acc = (rd(d, p + (0) - 5759) / 8 + rd(d, p + (0) - 5837) / 8 + rd(d, p + (0) - 5034) / 8 + rd(d, p + (0) - 6159) / 8 + rd(d, p + (0) - 5421) / 8 + rd(d, p + (0) - 6446) / 8 + rd(d, p + (0) - 6534) / 8 + rd(d, p + (0) - 6782) / 8 + rd(d, p + (0) - 6859) / 8 + rd(d, p + (0) - 7039) / 8 + rd(d, p + (0) - 7126) / 8 + rd(d, p + (0) - 7221) / 8 + rd(d, p + (0) - 8565) / 8 + rd(d, p + (0) - 7534) / 8 + rd(d, p + (0) - 7736) / 8 + rd(d, p + (0) - 7891) / 8 + rd(d, p + (0) - 9215) / 8 + rd(d, p + (0) - 8146) / 8 + rd(d, p + (0) - 8278) / 8 + -rd(d, p + (0) - 8447) / 4 + -rd(d, p + (0) - 7439) / 4 + -rd(d, p + (0) - 8700) / 4 + -rd(d, p + (0) - 8885) / 4 + -rd(d, p + (0) - 9012) / 4 + -rd(d, p + (0) - 9171) / 4 + -rd(d, p + (0) - 8034) / 4 + -rd(d, p + (0) - 9449) / 4 + -rd(d, p + (0) - 9513) / 4 + -rd(d, p + (0) - 9765) / 4 + -rd(d, p + (0) - 9867) / 4 + -rd(d, p + (0) - 10046) / 4 + -rd(d, p + (0) - 10110) / 4 + -rd(d, p + (0) - 10285) / 4 + -rd(d, p + (0) - 12074) / 4 + -rd(d, p + (0) - 10578) / 4 + -rd(d, p + (0) - 10734) / 4 + -rd(d, p + (0) - 11715) / 4 + -rd(d, p + (0) - 11023) / 4 + -rd(d, p + (0) - 11185) / 4 + rd(d, p + (0) - 11254) / 2 + rd(d, p + (0) - 11479) / 2 + rd(d, p + (0) - 11546) / 2 + rd(d, p + (0) - 10815) / 2 + rd(d, p + (0) - 11847) / 2 + rd(d, p + (0) - 10428) / 2 + rd(d, p + (0) - 12112) / 2 + rd(d, p + (0) - 12279) / 2 + rd(d, p + (0) - 12415) / 2 + rd(d, p + (0) - 12516) / 2 + rd(d, p + (0) - 12746) / 2 + rd(d, p + (0) - 12815) / 2 + rd(d, p + (0) - 13046) / 2 + rd(d, p + (0) - 13217) / 2 + rd(d, p + (0) - 13378) / 2 + rd(d, p + (0) - 13534) / 2) as i16 as i32;
    wr(d, p + (12), -acc);
    (left, right)
}

fn midiverb2_effect_94(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 1) + rd(d, p + (5) - 1) / 2 + rd(d, p + (16381) - 34) / 2) as i16 as i32;
    wr(d, p + (16381), -acc);
    acc = (-acc / 2 + rd(d, p + (16381) - 34) + rd(d, p + (16345) - 45) / 2) as i16 as i32;
    wr(d, p + (16345), -acc);
    acc = (-acc / 2 + rd(d, p + (16345) - 45) + rd(d, p + (16298) - 67) / 2) as i16 as i32;
    wr(d, p + (16298), -acc);
    acc = (-acc / 2 + rd(d, p + (16298) - 67) + rd(d, p + (16229) - 89) / 2) as i16 as i32;
    wr(d, p + (16229), -acc);
    acc = (-acc / 2 + rd(d, p + (16229) - 89) + rd(d, p + (16138) - 101) / 2) as i16 as i32;
    wr(d, p + (16138), -acc);
    acc = (-acc / 2 + rd(d, p + (16138) - 101) + rd(d, p + (16035) - 120) / 2) as i16 as i32;
    wr(d, p + (16035), -acc);
    acc = (-acc / 2 + rd(d, p + (16035) - 120)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (tmp_3) as i16 as i32;
    wr(d, p + (5), -acc);
    left = (rd(d, p + (5) - 0)) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_95(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (0) - 150)) as i16 as i32;
    tmp_3 = (acc) as i16 as i32;
    acc = (rd(d, p + (0) - 300)) as i16 as i32;
    tmp_5 = (acc) as i16 as i32;
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_96(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (-rd(d, p + (14261) - 1) / 4 + rd(d, p + (14264) - 1)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (-rd(d, p + (14264) - 1) / 4 + rd(d, p + (14261) - 1)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 3) * 3 / 2 + rd(d, p + (16089) - 15) / 2) as i16 as i32;
    wr(d, p + (16089), -acc);
    acc = (-acc / 2 + rd(d, p + (16089) - 15) + rd(d, p + (16072) - 22) / 2) as i16 as i32;
    wr(d, p + (16072), -acc);
    acc = (-acc / 2 + rd(d, p + (16072) - 22) + rd(d, p + (16048) - 29) / 2) as i16 as i32;
    wr(d, p + (16048), -acc);
    acc = (-acc / 2 + rd(d, p + (16048) - 29) + rd(d, p + (16017) - 45) / 2) as i16 as i32;
    wr(d, p + (16017), -acc);
    acc = (-acc / 2 + rd(d, p + (16017) - 45) + rd(d, p + (15970) - 61) / 2) as i16 as i32;
    wr(d, p + (15970), -acc);
    acc = (-acc / 2 + rd(d, p + (15970) - 61) + rd(d, p + (14997) - 105) / 2) as i16 as i32;
    wr(d, p + (14997), -acc);
    acc = (-acc / 2 + rd(d, p + (14997) - 105) + rd(d, p + (14890) - 149) / 2) as i16 as i32;
    wr(d, p + (14890), -acc);
    acc = (-acc / 2 + rd(d, p + (14890) - 149) + rd(d, p + (14739) - 213) / 2) as i16 as i32;
    wr(d, p + (14739), -acc);
    acc = (-acc / 2 + rd(d, p + (14739) - 213) + rd(d, p + (14524) - 258) / 2) as i16 as i32;
    wr(d, p + (14524), -acc);
    acc = (-acc / 2 + rd(d, p + (14524) - 258)) as i16 as i32;
    wr(d, p + (14264), -acc);
    acc = (rd(d, p + (0) - 3) * 3 / 2 + rd(d, p + (15210) - 17) / 2) as i16 as i32;
    wr(d, p + (15210), -acc);
    acc = (-acc / 2 + rd(d, p + (15210) - 17) + rd(d, p + (15191) - 25) / 2) as i16 as i32;
    wr(d, p + (15191), -acc);
    acc = (-acc / 2 + rd(d, p + (15191) - 25) + rd(d, p + (15164) - 35) / 2) as i16 as i32;
    wr(d, p + (15164), -acc);
    acc = (-acc / 2 + rd(d, p + (15164) - 35) + rd(d, p + (15127) - 52) / 2) as i16 as i32;
    wr(d, p + (15127), -acc);
    acc = (-acc / 2 + rd(d, p + (15127) - 52) + rd(d, p + (15073) - 74) / 2) as i16 as i32;
    wr(d, p + (15073), -acc);
    acc = (-acc / 2 + rd(d, p + (15073) - 74) + rd(d, p + (15907) - 88) / 2) as i16 as i32;
    wr(d, p + (15907), -acc);
    acc = (-acc / 2 + rd(d, p + (15907) - 88) + rd(d, p + (15817) - 123) / 2) as i16 as i32;
    wr(d, p + (15817), -acc);
    acc = (-acc / 2 + rd(d, p + (15817) - 123) + rd(d, p + (15692) - 174) / 2) as i16 as i32;
    wr(d, p + (15692), -acc);
    acc = (-acc / 2 + rd(d, p + (15692) - 174) + rd(d, p + (15516) - 304) / 2) as i16 as i32;
    wr(d, p + (15516), -acc);
    acc = (-acc / 2 + rd(d, p + (15516) - 304)) as i16 as i32;
    wr(d, p + (14261), -acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_97(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 5859)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 5859)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) + rd(d, p + (16380) - 5859) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_98(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 8789)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 8789)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) + rd(d, p + (16380) - 8789) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

fn midiverb2_effect_99(input: i32, d: &mut Dram, p: i32, lfo1: i32, lfo2: i32) -> (i16, i16) {
    let (mut left, mut right) = (0i16, 0i16);
    let mut acc = 0i32;
    let mut tmp_3 = 0i32;
    let mut tmp_5 = 0i32;
    wr(d, p + (0), input);
    acc = (rd(d, p + (16380) - 11718)) as i16 as i32;
    tmp_3 = (-acc) as i16 as i32;
    acc = (rd(d, p + (16380) - 11718)) as i16 as i32;
    tmp_5 = (-acc) as i16 as i32;
    acc = (rd(d, p + (0) - 2) + rd(d, p + (16380) - 11718) / 2) as i16 as i32;
    wr(d, p + (16380), acc);
    left = (tmp_5) as i16;
    right = (tmp_3) as i16;
    (left, right)
}

pub static MODEL_2: [Program; 100] = [
    Program { name: "Defeat", run: midiverb2_effect_0 },
    Program { name: "Small Bright .1 Sec", run: midiverb2_effect_1 },
    Program { name: "Small Bright .2 Sec", run: midiverb2_effect_2 },
    Program { name: "Small Bright .3 Sec", run: midiverb2_effect_3 },
    Program { name: "Medium Warm 1.1 Sec", run: midiverb2_effect_4 },
    Program { name: "Medium Bright .6 Sec", run: midiverb2_effect_5 },
    Program { name: "Large Bright 1.2 Sec", run: midiverb2_effect_6 },
    Program { name: "Large Dark 1.0 Sec", run: midiverb2_effect_7 },
    Program { name: "Medium Dark .6 Sec", run: midiverb2_effect_8 },
    Program { name: "Medium Dark .5 Sec", run: midiverb2_effect_9 },
    Program { name: "Medium Bright .6 Sec", run: midiverb2_effect_10 },
    Program { name: "Medium Bright .8 Sec", run: midiverb2_effect_11 },
    Program { name: "Large Warm 1.0 Sec", run: midiverb2_effect_12 },
    Program { name: "Large Warm 1.1 Sec", run: midiverb2_effect_13 },
    Program { name: "Medium Dark 1.0 Sec", run: midiverb2_effect_14 },
    Program { name: "Medium Bright 1.1 Sec", run: midiverb2_effect_15 },
    Program { name: "Medium Bright 1.15 Sec", run: midiverb2_effect_16 },
    Program { name: "Large Bright 1.6 Sec", run: midiverb2_effect_17 },
    Program { name: "Large Dark 1.7 Sec", run: midiverb2_effect_18 },
    Program { name: "Medum Bright 1.65 Sec", run: midiverb2_effect_19 },
    Program { name: "Medium Bright 1.9 Sec", run: midiverb2_effect_20 },
    Program { name: "Large Warm 2.2 Sec", run: midiverb2_effect_21 },
    Program { name: "Large Warm 1.75 Sec", run: midiverb2_effect_22 },
    Program { name: "Large Bright 1.45 Sec", run: midiverb2_effect_23 },
    Program { name: "Large Dark 2.2 Sec", run: midiverb2_effect_24 },
    Program { name: "Large Warm 2.3 Sec", run: midiverb2_effect_25 },
    Program { name: "Large Bright 2.4 Sec", run: midiverb2_effect_26 },
    Program { name: "Large Bright 2.5 Sec", run: midiverb2_effect_27 },
    Program { name: "Xlarge Warm 5.0 Sec", run: midiverb2_effect_28 },
    Program { name: "Xlarge Warm 15.0 Sec", run: midiverb2_effect_29 },
    Program { name: "Slow Gate 100 Msec", run: midiverb2_effect_30 },
    Program { name: "Slow Gate 200 Msec", run: midiverb2_effect_31 },
    Program { name: "Slow Gate 250 Msec", run: midiverb2_effect_32 },
    Program { name: "Slow Gate 400 Msec", run: midiverb2_effect_33 },
    Program { name: "Slow Gate 450 Msec", run: midiverb2_effect_34 },
    Program { name: "Fast Gate 150 Msec", run: midiverb2_effect_35 },
    Program { name: "Fast Gate 75 Msec", run: midiverb2_effect_36 },
    Program { name: "Fast Gate 200 Msec", run: midiverb2_effect_37 },
    Program { name: "Fast Gate 100 Msec", run: midiverb2_effect_38 },
    Program { name: "Fast Gate 175 Msec", run: midiverb2_effect_39 },
    Program { name: "Reverse 400 Msec", run: midiverb2_effect_40 },
    Program { name: "Reverse 450 Msec", run: midiverb2_effect_41 },
    Program { name: "Reverse 250 Msec", run: midiverb2_effect_42 },
    Program { name: "Reverse 200 Msec", run: midiverb2_effect_43 },
    Program { name: "Reverse 150 Msec", run: midiverb2_effect_44 },
    Program { name: "Bloom 1 8 Sec", run: midiverb2_effect_45 },
    Program { name: "Reverse 500 Msec", run: midiverb2_effect_46 },
    Program { name: "Reverse Regen. 2 Sec", run: midiverb2_effect_47 },
    Program { name: "Reverse Regen. 8 Sec", run: midiverb2_effect_48 },
    Program { name: "Bloom 2 7 Sec", run: midiverb2_effect_49 },
    Program { name: "Triggered Flange", run: midiverb2_effect_50 },
    Program { name: "Flange Pan 1", run: midiverb2_effect_51 },
    Program { name: "Flange Pan 2", run: midiverb2_effect_52 },
    Program { name: "Triggered Flange 2", run: midiverb2_effect_53 },
    Program { name: "Flange Pan 3", run: midiverb2_effect_54 },
    Program { name: "Flange Pan 4", run: midiverb2_effect_55 },
    Program { name: "Flange Pan 5", run: midiverb2_effect_56 },
    Program { name: "Triggered Flange 3", run: midiverb2_effect_57 },
    Program { name: "Flange Pan 6", run: midiverb2_effect_58 },
    Program { name: "Flange Pan 7", run: midiverb2_effect_59 },
    Program { name: "Light Chorus 1", run: midiverb2_effect_60 },
    Program { name: "Light Chorus 2", run: midiverb2_effect_61 },
    Program { name: "Medium Chorus 1", run: midiverb2_effect_62 },
    Program { name: "Medium Chorus 2", run: midiverb2_effect_63 },
    Program { name: "Medium Chorus 3", run: midiverb2_effect_64 },
    Program { name: "Deep Chorus 1", run: midiverb2_effect_65 },
    Program { name: "Deep Chorus 2", run: midiverb2_effect_66 },
    Program { name: "Deep Chorus 3", run: midiverb2_effect_67 },
    Program { name: "Fast Chorus 1", run: midiverb2_effect_68 },
    Program { name: "Fast Chorus 2", run: midiverb2_effect_69 },
    Program { name: "35 Msec", run: midiverb2_effect_70 },
    Program { name: "55 Msec", run: midiverb2_effect_71 },
    Program { name: "75 Msec", run: midiverb2_effect_72 },
    Program { name: "115 Msec", run: midiverb2_effect_73 },
    Program { name: "140 Msec", run: midiverb2_effect_74 },
    Program { name: "155 Msec", run: midiverb2_effect_75 },
    Program { name: "160 Msec", run: midiverb2_effect_76 },
    Program { name: "170 Msec", run: midiverb2_effect_77 },
    Program { name: "175 Msec", run: midiverb2_effect_78 },
    Program { name: "180 Msec", run: midiverb2_effect_79 },
    Program { name: "185 Msec", run: midiverb2_effect_80 },
    Program { name: "195 Msec", run: midiverb2_effect_81 },
    Program { name: "205 Msec", run: midiverb2_effect_82 },
    Program { name: "210 Msec", run: midiverb2_effect_83 },
    Program { name: "220 Msec", run: midiverb2_effect_84 },
    Program { name: "240 Msec", run: midiverb2_effect_85 },
    Program { name: "250 Msec", run: midiverb2_effect_86 },
    Program { name: "275 Msec", run: midiverb2_effect_87 },
    Program { name: "375 Msec", run: midiverb2_effect_88 },
    Program { name: "460 Msec", run: midiverb2_effect_89 },
    Program { name: "2 Tap Ambient", run: midiverb2_effect_90 },
    Program { name: "3 Tap Pan", run: midiverb2_effect_91 },
    Program { name: "Multitap", run: midiverb2_effect_92 },
    Program { name: "Multitap Reverse Pan", run: midiverb2_effect_93 },
    Program { name: "Thickener/frozen Flange", run: midiverb2_effect_94 },
    Program { name: "Stereo Generation", run: midiverb2_effect_95 },
    Program { name: "Stereo Generation Wide", run: midiverb2_effect_96 },
    Program { name: "Regenerated Delay 2 Sec", run: midiverb2_effect_97 },
    Program { name: "Regenerated Delay 3 Sec", run: midiverb2_effect_98 },
    Program { name: "Regenerated Delay 4 Sec", run: midiverb2_effect_99 },
];
