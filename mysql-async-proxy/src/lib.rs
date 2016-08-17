

#[derive(Debug)]
struct MySQLPacket<'a> {
    bytes: &'a [u8]
}

impl<'a> MySQLPacket<'a> {

    fn new(b: &'a [u8]) -> Self {
        MySQLPacket { bytes: b }
    }

    fn seq(&self) -> u8 { self.bytes[3] }
}

trait PacketHandler<'a> {
    fn transform_request(p: &MySQLPacket) -> Option<MySQLPacket<'a>>;
    fn transform_response(p: &MySQLPacket) -> Option<MySQLPacket<'a>>;
}



#[cfg(test)]
mod tests {

    use super::MySQLPacket;

    #[test]
    fn create_packet() {
        // COM_QUERY: select @@version_comment limit 1
        let bytes: &[u8] = &[
            0x21, 0x00, 0x00, 0x00, 0x03, 0x73, 0x65, 0x6c,
            0x65, 0x63, 0x74, 0x20, 0x40, 0x40, 0x76, 0x65,
            0x72, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x63, 0x6f,
            0x6d, 0x6d, 0x65, 0x6e, 0x74, 0x20, 0x6c, 0x69,
            0x6d, 0x69, 0x74, 0x20, 0x31
        ];

        let packet = MySQLPacket::new(bytes);

        print!("Packet = {:?}", packet);

        assert_eq!(0x00, packet.seq());
    }
}

// BELOW THIS LINE IS PROTOTYPING HOW A USER WOULD USE THE PROXY

struct Query {
    sql: String,
    // other stuff ... like instructions on how to decrypt corresponding result set
}

struct ConnState {
    queries: Vec<Query> // stack of queries sent to the server ... we can
}


