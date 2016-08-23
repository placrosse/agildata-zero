#[derive(Debug)]
pub struct MySQLPacket {
    pub bytes: Vec<u8>
}

impl MySQLPacket {

    pub fn parse_packet_length(header: &[u8]) -> usize {
        (((header[2] as u32) << 16) |
            ((header[1] as u32) << 8) |
            header[0] as u32) as usize
    }

    pub fn sequence_id(&self) -> u8 {
        self.bytes[3]
    }

    pub fn packet_type(&self) -> u8 {
        if self.bytes.len() > 4 {
            self.bytes[4]
        } else {
            0
        }
    }

}

pub struct MySQLPacketReader<'a> {
    payload: &'a [u8],
    pos: usize
}

impl<'a> MySQLPacketReader<'a> {

    pub fn new(packet: &'a MySQLPacket) -> Self {
        MySQLPacketReader { payload: &packet.bytes, pos: 4 }
    }

    /// read the length of a length-encoded field
    pub fn read_len(&mut self) -> usize {
        let n = self.payload[self.pos] as usize;
        self.pos += 1;

        match n {
            //NOTE: depending on context, 0xfb could mean null and 0xff could mean error
            0xfc | 0xfd | 0xfe => panic!("no support yet for length >= 251"),
            _ => n
        }
    }

    pub fn read_string(&mut self) -> Option<String> {
        match self.read_bytes() {
            Some(s) => Some(String::from_utf8(s.to_vec()).expect("Invalid UTF-8")),
            None => None
        }
    }

    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        match self.read_len() {
            0xfb => None,
            n @ _ => {
                let s = &self.payload[self.pos..self.pos+n];
                self.pos += n;
                let mut v : Vec<u8> = vec![];
                v.extend_from_slice(s);
                Some(v)
            }
        }
    }

}