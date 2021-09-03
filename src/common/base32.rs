/*
 * Functions for encoding and decoding byte arrays into base32.
 *
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

// Convert a 5-bit nickel, i.e. a value between 0 and 31 (inclusive), to
// a letter in the Base32 alphabet, which is an alphanumeric 8-bit character.
pub fn letter_from_nickel(value: u8) -> u8
{
	// Crockford Base32 alphabet where a = 10 and i, l, o and u are skipped.
	const ALPHABET: [u8; 32] = [
		b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b',
		b'c', b'd', b'e', b'f', b'g', b'h', b'j', b'k', b'm', b'n', b'p', b'q',
		b'r', b's', b't', b'v', b'w', b'x', b'y', b'z',
	];

	debug_assert!(value <= 31);
	ALPHABET[value as usize]
}

// Convert a (case insensitive) letter in the Crockford Base32 alphabet
// to a 5-bit nickel, i.e. a value between 0 and 31 (inclusive).
pub fn nickel_from_letter(x: u8) -> Result<u8, DecodeError>
{
	match x
	{
		b'0'..=b'9' => Ok(x - b'0'),
		// a = 10
		b'a'..=b'h' => Ok(x - b'a' + 10),
		// skip i
		b'j'..=b'k' => Ok(x - b'j' + 18),
		// skip l
		b'm'..=b'n' => Ok(x - b'm' + 20),
		// skip o
		b'p'..=b't' => Ok(x - b'p' + 22),
		// skip u
		b'v'..=b'z' => Ok(x - b'v' + 27),
		// continue with capitals
		b'A'..=b'H' => Ok(x - b'A' + 10),
		// skip I
		b'J'..=b'K' => Ok(x - b'J' + 18),
		// skip L
		b'M'..=b'N' => Ok(x - b'M' + 20),
		// skip O
		b'P'..=b'T' => Ok(x - b'P' + 22),
		// skip U
		b'V'..=b'Z' => Ok(x - b'V' + 27),
		// i, I, l and L are confused with 1
		b'i' | b'I' | b'l' | b'L' => Ok(1),
		// o and O are confused with 0
		b'o' | b'O' => Ok(0),
		// u and U are confused with v
		b'u' | b'U' => Ok(27),
		_ => Err(DecodeError::InvalidLetter { letter: x }),
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
	for character in word.iter_mut()
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
		*character = letter_from_nickel(nickel);
	}

	debug_assert!(datapos == datalength);
	String::from_utf8(word).unwrap()
}

// Convert a big-endian base32 string back into a big-endian base256 number. A
// byte array of size S=5N+K is encoded as a word of length l(S)=8N+f(K), where
// f(0) = 0, f(1) = 2, f(2) = 4, f(3) = 5 and f(4) = 7. Note that l() is
// injective, so we can determine the size S of a byte array given l(S).
pub fn decode(word: &str) -> Result<Vec<u8>, DecodeError>
{
	if !word.is_ascii()
	{
		return Err(DecodeError::NonAscii {
			source: word.to_string(),
		});
	}

	// Because decode is the inverse of encode, we want to determine how long
	// the original data array was, and we will drop the first few bits of this
	// word; we round down.
	let wordlength = word.len();
	let datalength = (wordlength * 5) / 8;
	let mut data = vec![0u8; datalength];

	// If necessary, we can drop bits from the front of the representation.
	// E.g. if we had encoded a single uint8_t, we are now decoding two
	// characters, which is 10 bits, but the first two bits should be zero.
	// If we have a single character, we treat it as 5 bits of garbage.
	let mut discarded = (wordlength * 5) % 8;

	// We have a buffer of between 0 and 12 bits to draw from; we use the
	// most significant bits, so bitpositions 12, ..., 15 will always be zero.
	// We add five bits each time and we take the eight most significant bits
	// whenever have less than eight bits remaining.
	let mut nbits: i8 = 0;
	let mut buffer: u16 = 0;

	// Decode the word one character at a time.
	let mut datapos = 0;
	for x in word.bytes()
	{
		let value: u8 = nickel_from_letter(x)?;
		debug_assert!(value <= 31);

		if discarded >= 5
		{
			// This entire character is unusable and should be zero.
			if value > 0
			{
				return Err(DecodeError::NonZeroLeadingBits {
					source: word.to_string(),
				});
			}
			discarded -= 5;
			continue;
		}
		else if discarded > 0
		{
			// The leading bits should be zero.
			if value >= 1 << (5 - discarded)
			{
				return Err(DecodeError::NonZeroLeadingBits {
					source: word.to_string(),
				});
			}

			// The leading zeroes are non-significant.
			nbits -= discarded as i8;
			discarded = 0;
		}

		// Add the fresh bits.
		buffer |= (value as u16) << (11 - nbits);
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
		letter: u8
	},
	NonZeroLeadingBits
	{
		source: String
	},
	WordTooLong
	{
		source: String,
		max_length_in_bits: usize,
	},
	WordTooShort
	{
		source: String,
		min_length_in_bits: usize,
	},
	NonAscii
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
			DecodeError::WordTooLong {
				source,
				max_length_in_bits,
			} => write!(
				f,
				"too many characters in '{}' for {} bits of data",
				source, max_length_in_bits
			),
			DecodeError::WordTooShort {
				source,
				min_length_in_bits,
			} => write!(
				f,
				"not enough characters in '{}' for {} bits of data",
				source, min_length_in_bits
			),
			DecodeError::NonAscii { source } =>
			{
				write!(f, "non-ASCII characters in '{}'", source)
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
			assert_eq!(nickel_from_letter(letter_from_nickel(nickel))?, nickel);
		}
		Ok(())
	}

	#[test]
	fn test_case_insensitivity() -> Result<(), DecodeError>
	{
		for letter in b'a'..=b'z'
		{
			let upper = letter.to_ascii_uppercase();
			assert_eq!(nickel_from_letter(upper)?, nickel_from_letter(letter)?);
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

	#[test]
	fn test_empty() -> Result<(), DecodeError>
	{
		let encoded = encode(&[]);
		assert_eq!(encoded.len(), 0);
		let decoded = decode(&encoded)?;
		assert_eq!(decoded.len(), 0);
		Ok(())
	}

	#[test]
	fn test_len() -> Result<(), DecodeError>
	{
		for len in 1..=20
		{
			let mut data = vec![0u8; len];
			for x in 0..=255
			{
				data[0] = x;
				let encoded = encode(&data);
				let decoded = decode(&encoded)?;
				assert_eq!(decoded, data, "(encoded = {})", encoded);
				assert_eq!(encode(&decoded), encoded,);
			}
		}
		Ok(())
	}

	#[test]
	fn test_inverse_len() -> Result<(), DecodeError>
	{
		let len = 256;
		let text = "0".repeat(len);
		for n in 0..=len
		{
			decode(&text[0..n])?;
		}
		Ok(())
	}

	#[test]
	fn test_garbage()
	{
		assert!(decode("abc ").is_err());
		assert!(decode("abc\0").is_err());
	}

	#[test]
	fn test_leading_zeroes() -> Result<(), DecodeError>
	{
		{
			let decoded = decode("0")?;
			assert_eq!(decoded.len(), 0);
		}
		{
			let decoded = decode("7z")?;
			assert_eq!(decoded, [255u8]);
		}
		{
			let decoded = decode("07z")?;
			assert_eq!(decoded, [255u8]);
		}
		Ok(())
	}

	#[test]
	fn test_nonzero_leading_bits()
	{
		assert!(decode("a").is_err());
		assert!(decode("zz").is_err());
	}
}
