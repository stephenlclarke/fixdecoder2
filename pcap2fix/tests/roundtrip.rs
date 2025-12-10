use assert_cmd::Command;

/// Build a minimal FIX message with correct BodyLength/Checksum using the given delimiter.
fn build_fix_message(delim: u8) -> Vec<u8> {
    let d = delim as char;
    let body = format!("35=0{d}");
    let body_len = body.len();
    let mut msg = format!("8=FIX.4.2{d}9={body_len}{d}{body}").into_bytes();
    let checksum: u8 = msg.iter().fold(0u16, |acc, b| acc + *b as u16) as u8;
    msg.extend_from_slice(format!("10={:03}{}", checksum, d).as_bytes());
    msg
}

/// Construct a tiny PCAP (classic) containing one Ethernet/IPv4/TCP packet with the FIX payload.
fn build_pcap(payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();

    // PCAP global header (little-endian, Ethernet linktype)
    buf.extend_from_slice(&0xa1b2c3d4u32.to_le_bytes()); // magic
    buf.extend_from_slice(&0x0002u16.to_le_bytes()); // version major
    buf.extend_from_slice(&0x0004u16.to_le_bytes()); // version minor
    buf.extend_from_slice(&0u32.to_le_bytes()); // thiszone
    buf.extend_from_slice(&0u32.to_le_bytes()); // sigfigs
    buf.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
    buf.extend_from_slice(&1u32.to_le_bytes()); // network = Ethernet

    // Build packet bytes
    let mut pkt = Vec::new();
    // Ethernet
    pkt.extend_from_slice(&[0, 1, 2, 3, 4, 5]); // dst
    pkt.extend_from_slice(&[6, 7, 8, 9, 10, 11]); // src
    pkt.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4
                                          // IPv4 header
    let ip_header_len = 20u16;
    let tcp_header_len = 20u16;
    let total_len = ip_header_len + tcp_header_len + payload.len() as u16;
    pkt.extend_from_slice(&[0x45, 0x00]); // version/IHL, DSCP
    pkt.extend_from_slice(&total_len.to_be_bytes());
    pkt.extend_from_slice(&[0x00, 0x00]); // identification
    pkt.extend_from_slice(&[0x40, 0x00]); // flags/frag offset
    pkt.extend_from_slice(&[64]); // TTL
    pkt.extend_from_slice(&[6]); // protocol TCP
    pkt.extend_from_slice(&[0x00, 0x00]); // checksum (omitted)
    pkt.extend_from_slice(&[10, 0, 0, 1]); // src IP
    pkt.extend_from_slice(&[10, 0, 0, 2]); // dst IP
                                           // TCP header
    let src_port: u16 = 40000;
    let dst_port: u16 = 12083;
    pkt.extend_from_slice(&src_port.to_be_bytes());
    pkt.extend_from_slice(&dst_port.to_be_bytes());
    pkt.extend_from_slice(&1u32.to_be_bytes()); // seq
    pkt.extend_from_slice(&0u32.to_be_bytes()); // ack
    pkt.extend_from_slice(&[0x50, 0x18]); // data offset=5, flags=PSH+ACK
    pkt.extend_from_slice(&0xffffu16.to_be_bytes()); // window
    pkt.extend_from_slice(&[0x00, 0x00]); // checksum (omitted)
    pkt.extend_from_slice(&[0x00, 0x00]); // urgent ptr
                                          // Payload
    pkt.extend_from_slice(payload);

    // PCAP packet header
    let pkt_len = pkt.len() as u32;
    buf.extend_from_slice(&0u32.to_le_bytes()); // ts_sec
    buf.extend_from_slice(&0u32.to_le_bytes()); // ts_usec
    buf.extend_from_slice(&pkt_len.to_le_bytes()); // incl_len
    buf.extend_from_slice(&pkt_len.to_le_bytes()); // orig_len

    buf.extend_from_slice(&pkt);
    buf
}

#[test]
fn pcap_roundtrip_matches_expected_output() {
    let delim = 0x01;
    let msg = build_fix_message(delim);
    let pcap_bytes = build_pcap(&msg);
    let expected_output = {
        let mut v = msg.clone();
        v.push(b'\n');
        v
    };

    let bin = assert_cmd::cargo::cargo_bin!("pcap2fix");
    Command::new(bin)
        .args(["--input", "-", "--port", "12083"])
        .write_stdin(pcap_bytes)
        .assert()
        .success()
        .stdout(expected_output);
}
