#[test]
fn dump() {
    for bytes in [&[0xa5u8][..], &[0x67, 0xa4][..], &[0xf3, 0xab][..], &[0xf2, 0xae][..]] {
        let mut d = iced_x86::Decoder::with_ip(64, bytes, 0x1000, iced_x86::DecoderOptions::NONE);
        let i = d.decode();
        let ops: Vec<String> = (0..i.op_count()).map(|k| format!("{:?}/{:?}", i.op_kind(k), i.op_register(k))).collect();
        println!(
            "{:02x?} mn={:?} ops={:?} base={:?} seg={:?} rep={} repne={} repe={} code={:?}",
            bytes,
            i.mnemonic(),
            ops,
            i.memory_base(),
            i.memory_segment(),
            i.has_rep_prefix(),
            i.has_repne_prefix(),
            i.has_repe_prefix(),
            i.code()
        );
    }
}
