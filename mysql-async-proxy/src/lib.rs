

struct MySQLPacket<'a> {
    bytes: &'a [u8]
}

impl<'a> MySQLPacket<'a> {
    fn seq(&self) -> u8 { self.bytes[3] }

}


