/* Functions for encoding and decoding byte arrays into base32. */

// Convert a 5-bit nickel, i.e. a value between 0 and 31 (inclusive), to
// a letter in the Base32 alphabet, which is an alphanumeric 8-bit character.
pub fn char_from_nickel(value: u8) -> u8
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

	debug_assert!(value <= 31);
	return ALPHABET[value as usize];
}

// Convert a big-endian bitstring to a big-endian base32 alphanumeric string.
pub fn encode(data: &[u8]) -> String
{
	// Calculate the length of the resulting word, rounding up.
	// E.g. a single byte takes 2 * 5 bits, five byte take exactly 8 * 5 bits.
	let datalength = data.len();
	let wordlength = (datalength * 8 + 4) / 5;
	let mut word = vec![0u8; wordlength];

	// We have a buffer of between 0 and 12 bits to draw from; we use the
	// most-significant bits, so bitpositions 12, ..., 15 will always be zero.
	// We take the five most-significant bits each time and add eight more
	// bits whenever have less than five bits remaining.
	let mut buffer: u16 = 0;

	// If needed, we prepend zeroes to the front of the big-endian bitstring.
	// E.g. if we encode a single byte, we use 10 bits as 2 * 5 = 10 >= 1 * 8,
	// so we prepend 2 zeroes to the big-endian bitstring.
	// If we want to encode five bytes, we use 40 bits as 8 * 5 = 40 == 5 * 8,
	// so we do not need to prepend anything.
	let mut nbits = (5 - (datalength * 8) % 5) % 5;

	// Create the word from left to right.
	let mut datapos = 0;
	for wordpos in 0..wordlength
	{
		// Do we need to add fresh bits?
		if nbits < 5
		{
			// Move the bits to bitpositions nbits, ..., nbits + 7, and
			// thus after the nbits most-significant bits in the buffer.
			buffer |= (data[datapos] as u16) << (8 - nbits);
			datapos += 1;
			nbits += 8;
		}

		// Consume the five most-significant bits.
		let nickel = (buffer >> 11) as u8;
		buffer <<= 5;
		nbits -= 5;

		// Turn those five bits into the next character.
		word[wordpos] = char_from_nickel(nickel);
	}

	return String::from_utf8(word).unwrap();
}
