/* Keycode */

use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Keycode(pub u64);

// Create a lossy keycode by interleaving a 16 bit key and the 44 least-
// significant bits of a data element into a 60 bit number that can be encoded
// as a big-endian base32-word of length 12 (since 32^12 = 2^60).
pub fn keycode(key: u16, data: u64) -> Keycode
{
	// The left-most 5 bits of the keycode are based on the 5 most-significant
	// bits of the key; the next 5 bots are based on the next 4 bits of the
	// key and the 1 most-significant bit of the data; ...; the six right-most
	// groups of 5 bits are based on the 30 least-significant bits of the data.
	const BITES: [i8; 12] = [5, 4, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0];

	let mut k_bitstring = key as u64;
	let mut d_bitstring = data;
	let mut result: u64 = 0;

	// We fill the word from right (least-significant) to left (most-sign.).
	for i in (0..12).rev()
	{
		// Consume the `bite` least-significant bits from the key and
		// the `5 - bite` least-significant bits from the data, then combine
		// them to be the next 5 bits.
		let bite = BITES[i];
		let k_mask = (1u64 << bite) - 1;
		let d_mask = (1u64 << (5 - bite)) - 1;
		let k_bits = k_bitstring & k_mask;
		let d_bits = d_bitstring & d_mask;
		let r_bits = (k_bits << (5 - bite)) | d_bits;
		k_bitstring >>= bite;
		d_bitstring >>= 5 - bite;
		// Then convert the 5-bit index to Base32.
		result = (result << 5) | r_bits;
	}

	Keycode(result)
}

impl fmt::Display for Keycode
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		// Crockford Base32 alphabet where a = 10 and i, l, o and u are skipped.
		const ALPHABET: [u8; 32] = [
			'0' as u8, '1' as u8, '2' as u8, '3' as u8, '4' as u8, '5' as u8,
			'6' as u8, '7' as u8, '8' as u8, '9' as u8, 'a' as u8, 'b' as u8,
			'c' as u8, 'd' as u8, 'e' as u8, 'f' as u8, 'g' as u8, 'h' as u8,
			'j' as u8, 'k' as u8, 'm' as u8, 'n' as u8, 'p' as u8, 'q' as u8,
			'r' as u8, 's' as u8, 't' as u8, 'v' as u8, 'w' as u8, 'x' as u8,
			'y' as u8, 'z' as u8,
		];

		let mut bits = self.0;
		let mut word = vec![0u8; 12];

		// We fill the word from right (least-significant) to left (most-sign.).
		for i in (0..12).rev()
		{
			// Consume the 5 least-significant bits from the keycode
			// as a 5-bit index, i.e. a number from 0 to 31 (inclusive).
			let index = (bits & 0x1F) as usize;
			bits >>= 5;
			// Then convert the 5-bit index to Base32.
			word[i] = ALPHABET[index];
		}

		let x = String::from_utf8(word).unwrap();
		write!(f, "{}", x)
	}
}
