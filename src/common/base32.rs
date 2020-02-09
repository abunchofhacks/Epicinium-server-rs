/* Functions for encoding and decoding byte arrays into base32. */

// Convert a 5-bit nickel, i.e. a value between 0 and 31 (inclusive), to
// a letter in the Base32 alphabet, which is an alphanumeric 8-bit character.
pub fn letter_from_nickel(value: u8) -> u8
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
	ALPHABET[value as usize]
}

// Convert a (case insensitive) letter in the Crockform Base32 alphabet
// to a 5-bit nickel, i.e. a value between 0 and 31 (inclusive).
fn nickel_from_letter(x: u8) -> Result<u8, DecodeError>
{
	if x >= b'0' && x <= b'9'
	{
		Ok(x - b'0')
	}
	// a = 10
	else if x >= b'a' && x <= b'h'
	{
		Ok(x - b'a' + 10)
	}
	// skip i
	else if x >= b'j' && x <= b'k'
	{
		Ok(x - b'j' + 18)
	}
	// skip l
	else if x >= b'm' && x <= b'n'
	{
		Ok(x - b'm' + 20)
	}
	// skip o
	else if x >= b'p' && x <= b't'
	{
		Ok(x - b'p' + 22)
	}
	// skip u
	else if x >= b'v' && x <= b'z'
	{
		Ok(x - b'v' + 27)
	}
	// continue with capitals
	else if x >= b'A' && x <= b'H'
	{
		Ok(x - b'A' + 10)
	}
	// skip I
	else if x >= b'J' && x <= b'K'
	{
		Ok(x - b'J' + 18)
	}
	// skip L
	else if x >= b'M' && x <= b'N'
	{
		Ok(x - b'M' + 20)
	}
	// skip O
	else if x >= b'P' && x <= b'T'
	{
		Ok(x - b'P' + 22)
	}
	// skip U
	else if x >= b'V' && x <= b'Z'
	{
		Ok(x - b'V' + 27)
	}
	// i and I are confused with 1
	else if x == b'i' || x == b'I'
	{
		Ok(1)
	}
	// l and L are confused with 1
	else if x == b'l' || x == b'L'
	{
		Ok(1)
	}
	// o and O are confused with 0
	else if x == b'o' || x == b'O'
	{
		Ok(0)
	}
	// u and U are confused with v
	else if x == b'u' || x == b'U'
	{
		Ok(27)
	}
	else
	{
		Err(DecodeError::InvalidLetter { letter: x as char })
	}
}

fn nickel_from_char(c: char) -> Result<u8, DecodeError>
{
	if c >= ' ' && c <= '~'
	{
		nickel_from_letter(c as u8)
	}
	else
	{
		Err(DecodeError::InvalidLetter { letter: c })
	}
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
		word[wordpos] = letter_from_nickel(nickel);
	}

	debug_assert!(datapos == datalength);
	String::from_utf8(word).unwrap()
}

// Convert a big-endian base32 string back into a big-endian base256 number. A
// byte array of size S=5N+K is encoded as a word of length l(S)=8N+f(K), where
// f(0) = 0, f(1) = 2, f(2) = 4, f(3) = 5 and f(4) = 7. Note that l() is
// surjective, so we can determine the size S of a byte array given l(S).
pub fn decode(word: &str) -> Result<Vec<u8>, DecodeError>
{
	// Because decode is the inverse of encode, we want to determine how long
	// the original data array was, and we will drop the first few bits of this
	// word; we round down.
	let wordlength = word.len();
	let datalength = (wordlength * 5) / 8;
	let mut data = vec![0u8; datalength];

	// If necessary, we can drop bits from the front of the representation.
	// E.g. if we had encoded a single uint8_t, we are now decoding two
	// characters, which is 10 bits, but the first two bits should be zero.
	let discarded = (wordlength * 5) % 8;
	debug_assert!(discarded < 5);

	// We have a buffer of between 0 and 12 bits to draw from; we use the
	// most significant bits, so bitpositions 12, ..., 15 will always be zero.
	// We add five bits each time and we take the eight most significant bits
	// whenever have less than eight bits remaining.
	let mut nbits = 0;
	let mut buffer: u16 = 0;

	// Decode the word one character at a time.
	let mut datapos = 0;
	for (i, c) in word.chars().enumerate()
	{
		let value: u8 = nickel_from_char(c)?;
		debug_assert!(value <= 31);

		let mut freshbits = value as u16;
		if i == 0 && discarded > 0
		{
			// The leading bits should be zero.
			if value >= 1 << (5 - discarded)
			{
				return Err(DecodeError::NonZeroLeadingBits {
					source: word.to_string(),
				});
			}

			// The leading zeroes are non-significant.
			freshbits <<= discarded;
		}

		// Add the fresh bits.
		buffer |= freshbits << (11 - nbits);
		nbits += 5;

		// Can we consume eight bits?
		if nbits >= 8
		{
			// Consume the eight left-most bits.
			data[datapos] = (buffer >> 8) as u8;
			datapos += 1;
			buffer <<= 8;
			nbits -= 8;
		}
	}

	debug_assert!(datapos == datalength);
	Ok(data)
}

#[derive(Debug)]
pub enum DecodeError
{
	InvalidLetter
	{
		letter: char
	},
	NonZeroLeadingBits
	{
		source: String
	},
}

impl std::error::Error for DecodeError {}

impl std::fmt::Display for DecodeError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			DecodeError::InvalidLetter { letter } =>
			{
				write!(f, "invalid non-Base32 character '{}'", letter)
			}
			DecodeError::NonZeroLeadingBits { source } =>
			{
				write!(f, "non-zero leading bits in '{}'", source)
			}
		}
	}
}

#[cfg(test)]
mod tests
{
	use super::*;

	#[test]
	fn test_inverse() -> Result<(), DecodeError>
	{
		for nickel in 0..=31
		{
			assert_eq!(nickel, nickel_from_letter(letter_from_nickel(nickel))?);
		}
		Ok(())
	}

	#[test]
	fn test_case_insensitivity() -> Result<(), DecodeError>
	{
		for letter in b'a'..=b'z'
		{
			let upper = letter.to_ascii_uppercase();
			assert_eq!(nickel_from_letter(letter)?, nickel_from_letter(upper)?);
		}
		Ok(())
	}

	#[test]
	fn test_confusion() -> Result<(), DecodeError>
	{
		assert_eq!(nickel_from_letter(b'i')?, nickel_from_letter(b'1')?);
		assert_eq!(nickel_from_letter(b'l')?, nickel_from_letter(b'1')?);
		assert_eq!(nickel_from_letter(b'o')?, nickel_from_letter(b'0')?);
		assert_eq!(nickel_from_letter(b'u')?, nickel_from_letter(b'v')?);
		Ok(())
	}
}
