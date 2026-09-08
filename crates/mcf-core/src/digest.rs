use core::fmt;

const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUNDS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn hex(&self) -> String {
        const HEX: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ];
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            for nibble in [byte >> 4, byte & 0x0f] {
                out.push(*HEX.get(usize::from(nibble)).unwrap_or(&'?'));
            }
        }
        out
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

#[derive(Debug, Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length_bits: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: INITIAL,
            buffer: [0; 64],
            buffered: 0,
            length_bits: 0,
        }
    }

    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length_bits = self
            .length_bits
            .wrapping_add((bytes.len() as u64).wrapping_mul(8));
        while !bytes.is_empty() {
            let room = 64 - self.buffered;
            let take = room.min(bytes.len());
            let (head, tail) = bytes.split_at(take);
            self.buffer
                .get_mut(self.buffered..self.buffered + take)
                .unwrap_or(&mut [])
                .copy_from_slice(head);
            self.buffered += take;
            bytes = tail;
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
    }

    #[must_use]
    pub fn finish(mut self) -> Digest {
        let length = self.length_bits;
        self.update_raw(&[0x80]);
        while self.buffered != 56 {
            self.update_raw(&[0]);
        }
        self.update_raw(&length.to_be_bytes());

        let mut digest = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            let bytes = word.to_be_bytes();
            let at = index * 4;
            if let Some(slot) = digest.get_mut(at..at + 4) {
                slot.copy_from_slice(&bytes);
            }
        }
        Digest(digest)
    }

    fn update_raw(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if let Some(slot) = self.buffer.get_mut(self.buffered) {
                *slot = *byte;
            }
            self.buffered += 1;
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
    }

    #[allow(clippy::many_single_char_names)]
    fn compress(&mut self, block: &[u8; 64]) {
        let mut schedule = [0_u32; 64];
        for (index, slot) in schedule.iter_mut().enumerate().take(16) {
            let at = index * 4;
            let word = block.get(at..at + 4).unwrap_or(&[0, 0, 0, 0]);
            *slot = u32::from_be_bytes([
                *word.first().unwrap_or(&0),
                *word.get(1).unwrap_or(&0),
                *word.get(2).unwrap_or(&0),
                *word.get(3).unwrap_or(&0),
            ]);
        }
        for index in 16..64 {
            let two = *schedule.get(index - 2).unwrap_or(&0);
            let seven = *schedule.get(index - 7).unwrap_or(&0);
            let fifteen = *schedule.get(index - 15).unwrap_or(&0);
            let sixteen = *schedule.get(index - 16).unwrap_or(&0);
            let s0 = fifteen.rotate_right(7) ^ fifteen.rotate_right(18) ^ (fifteen >> 3);
            let s1 = two.rotate_right(17) ^ two.rotate_right(19) ^ (two >> 10);
            if let Some(slot) = schedule.get_mut(index) {
                *slot = sixteen
                    .wrapping_add(s0)
                    .wrapping_add(seven)
                    .wrapping_add(s1);
            }
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(*ROUNDS.get(index).unwrap_or(&0))
                .wrapping_add(*schedule.get(index).unwrap_or(&0));
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[must_use]
pub fn sha256(bytes: &[u8]) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests;
