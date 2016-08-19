extern crate byteorder;

#[allow(non_camel_case_types, dead_code)] // FIXME: BOTH OF THESE SHOULDN'T HAVE TO EXIST!!!
#[derive(Debug)]
enum MySQLPacket {
    COM_Quit,
    COM_InitDB { db: String },
    COM_Query { query: String},
    COM_Ping,
    OK_Packet,
    Err_Packet,
    EOF_Packet
}

trait ProtocolHandler {
    fn parse(&self, bytes: &[u8]) -> Result<MySQLPacket, &'static str>;
}

// fn parse_string(bytes: &[u8]) -> String {
//     String::from_utf8(bytes.to_vec()).expect("Invalid UTF-8")
// }


#[cfg(test)]
mod tests {

    struct MockProtocolHandler {

    }

    impl ProtocolHandler for MockProtocolHandler {

        fn parse(&self, bytes: &[u8]) -> Result<MySQLPacket, &'static str> {

            // first three bytes denote packet length (little endian)
            let packet_len: u32 =
                ((bytes[2] as u32) << 16) |
                ((bytes[1] as u32) << 8) |
                bytes[0] as u32;
            print!("packet_len = {}", packet_len);

            // next byte is sequence_id
            let _sequence_id = bytes[3];

            // then payload
            match bytes[4] {
                0x01 => Ok(MySQLPacket::COM_Quit),
                0x02 => Ok(MySQLPacket::COM_InitDB {
                    db: parse_string(&bytes[5 as usize .. (packet_len+4) as usize])
                }),
                0x03 => Ok(MySQLPacket::COM_Query {
                    query: parse_string(&bytes[5 as usize .. (packet_len+4) as usize])
                }),
                0x0e => Ok(MySQLPacket::COM_Ping),
                _ => Err("Unsupported packet type")
            }

        }
    }

    #[test]
    fn mock() {

        let handler = MockProtocolHandler {};

        // COM_QUERY: select @@version_comment limit 1
        let packet: &[u8] = &[
           0x21, 0x00, 0x00, 0x00, 0x03, 0x73, 0x65, 0x6c,
           0x65, 0x63, 0x74, 0x20, 0x40, 0x40, 0x76, 0x65,
           0x72, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x63, 0x6f,
           0x6d, 0x6d, 0x65, 0x6e, 0x74, 0x20, 0x6c, 0x69,
           0x6d, 0x69, 0x74, 0x20, 0x31
        ];

        let packet = handler.parse(packet).unwrap();

        print!("Packet = {:?}", packet);
    }

}
