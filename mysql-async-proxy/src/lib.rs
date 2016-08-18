use std::str;

#[derive(Debug)]
struct MySQLPacket<'a> {
    bytes: &'a [u8]
}

impl<'a> MySQLPacket<'a> {

    fn new(b: &'a [u8]) -> Self {
        MySQLPacket { bytes: b }
    }

    fn packet_len(&self) -> usize {
        (((self.bytes[2] as u32) << 16) |
         ((self.bytes[1] as u32) << 8)  |
           self.bytes[0] as u32) as usize
    }

    fn sequence_id(&self) -> u8 { self.bytes[3] }

    fn packet_type(&self) -> u8 { self.bytes[4] }

    fn payload(&self) -> &[u8] { &self.bytes[4..] }


}

#[derive(Debug)]
struct MySQLPacketReader<'a> {
    payload: &'a MySQLPacket<'a>,
    pos: usize
}

impl<'a> MySQLPacketReader<'a> {

    fn new(p: &'a MySQLPacket) -> Self {
        MySQLPacketReader { payload: p, pos: 4 }
    }

    fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn read_len(&mut self) -> usize {
        let n = self.payload.bytes[self.pos] as usize;
        self.pos += 1;

        match n {
            //NOTE: depending on context, 0xfb could mean null and 0xff could mean error
            0xfc | 0xfd | 0xfe => panic!("no support yet for length >= 251"),
            _ => n
        }
    }

    fn read_lenenc_str(&'a mut self) -> Option<&'a str> {
        match self.read_lenenc_bytes() {
            Some(s) => Some(str::from_utf8(s).unwrap()),
            None => None
        }
    }

    fn read_lenenc_bytes(&mut self) -> Option<&[u8]> {
        println!("read_len_bytes BEGIN pos={}", self.pos);

        match self.read_len() {
            0xfb => None,
            n @ _ => {
                println!("read_len_bytes str_len={}", n);
                let s = &self.payload.bytes[self.pos..self.pos+n];
                self.pos += n;
                Some(s)
            }
        }
    }

}

#[cfg(test)]
mod tests {

    use super::{MySQLPacket, MySQLPacketReader};

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

        assert_eq!(0x21, packet.packet_len());
        assert_eq!(0x00, packet.sequence_id());
        assert_eq!(0x03, packet.packet_type());

        let mut reader = MySQLPacketReader::new(&packet);

        //TODO: this test is incorrect .. COM_QUERY does not use length-encoded strings
//        reader.skip(1); // packet type
//        assert_eq!(String::from("select @@version_comment limit 1"), reader.read_lenenc_str().unwrap());
    }
}

// BELOW THIS LINE IS PROTOTYPING HOW A USER WOULD USE THE PROXY

