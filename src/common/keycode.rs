/* Keycode */

// Create a lossy keycode by interleaving a 16 bit key and a 44 bit data element
// into a 60 bit number that is then encoded as a big-endian base32-word of
// length 12 (since 32^12 = 2^60).
pub fn keycode(key: u16, data: u64) -> String
{
	// Crockford's Base32 alphabet where a = 10 and i, l, o and u are skipped.
	const ALPHABET: [u8; 32] = [
		'0' as u8, '1' as u8, '2' as u8, '3' as u8, '4' as u8, '5' as u8,
		'6' as u8, '7' as u8, '8' as u8, '9' as u8, 'a' as u8, 'b' as u8,
		'c' as u8, 'd' as u8, 'e' as u8, 'f' as u8, 'g' as u8, 'h' as u8,
		'j' as u8, 'k' as u8, 'm' as u8, 'n' as u8, 'p' as u8, 'q' as u8,
		'r' as u8, 's' as u8, 't' as u8, 'v' as u8, 'w' as u8, 'x' as u8,
		'y' as u8, 'z' as u8,
	];

	// The left-most character of the word is based on the 5 most-significant
	// bits of the key; the next character is based on the next 4 bits of the
	// key and the 1 most-significant bit of the data; ...; the six right-most
	// characters are based on the 30 least-significant bits of the data.
	const BITES: [i8; 12] = [5, 4, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0];

	let mut k_bitstring = key;
	let mut d_bitstring = data;
	let mut word = vec![0u8; 12];

	// We fill the word from right (least-significant) to left (most-sign.).
	for i in (0..12).rev()
	{
		// Consume the `bite` least-significant bits from the key and
		// the `5 - bite` least-significant bits from the data, then combine
		// them to a 5-bit index, i.e. a number from 0 to 31 (inclusive).
		// The integer sizes are a bit nonsense because we basically want u5,
		// but d_mask is u64 so the `as usize` truncates nothing but zeroes.
		let bite = BITES[i];
		let k_mask = (1u16 << bite) - 1;
		let d_mask = (1u64 << (5 - bite)) - 1;
		let k_bits = (k_bitstring & k_mask) as usize;
		let d_bits = (d_bitstring & d_mask) as usize;
		let index = (k_bits << (5 - bite)) | d_bits;
		k_bitstring >>= bite;
		d_bitstring >>= 5 - bite;
		// Then convert the 5-bit index to Base32.
		word[i] = ALPHABET[index];
	}

	String::from_utf8(word).unwrap()
}
