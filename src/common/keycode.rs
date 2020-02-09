/* Keycode */

use crate::common::base32;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

	debug_assert!(result < (1 << 60));
	Keycode(result)
}

impl std::fmt::Display for Keycode
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		let mut bits = self.0;
		let mut word = vec![0u8; 12];

		// We fill the word from right (least-significant) to left (most-sign.).
		for i in (0..12).rev()
		{
			// Consume the 5 least-significant bits from the keycode
			// as a 5-bit index, i.e. a number from 0 to 31 (inclusive).
			let nickel = (bits & 0x1F) as u8;
			bits >>= 5;
			// Then convert the 5-bit index to Base32.
			word[i] = base32::letter_from_nickel(nickel);
		}

		let x = String::from_utf8(word).unwrap();
		write!(f, "{}", x)
	}
}

impl std::str::FromStr for Keycode
{
	type Err = base32::DecodeError;

	fn from_str(s: &str) -> Result<Keycode, base32::DecodeError>
	{
		let mut bits: u64 = 0;

		if !s.is_ascii()
		{
			return Err(base32::DecodeError::NonAscii {
				source: s.to_string(),
			});
		}
		else if s.len() < 12
		{
			return Err(base32::DecodeError::WordTooShort {
				source: s.to_string(),
				min_length_in_bits: 60,
			});
		}
		else if s.len() > 12
		{
			return Err(base32::DecodeError::WordTooLong {
				source: s.to_string(),
				max_length_in_bits: 60,
			});
		}

		// We parse the word from left (most-significant) to right (least).
		for x in s.bytes()
		{
			let nickel: u8 = base32::nickel_from_letter(x)?;
			debug_assert!(nickel <= 31);

			// Push in 5 bits.
			bits <<= 5;
			bits |= nickel as u64;
		}

		Ok(Keycode(bits))
	}
}

impl Serialize for Keycode
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Keycode
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		std::str::FromStr::from_str(&s).map_err(::serde::de::Error::custom)
	}
}

#[cfg(test)]
mod tests
{
	use super::*;

	#[test]
	fn test_inverse() -> Result<(), base32::DecodeError>
	{
		for _ in 0..1000
		{
			let key: u16 = rand::random();
			let data: u64 = rand::random();
			let keycode = keycode(key, data);
			let repr = keycode.to_string();
			let base32_word = base32::encode(&keycode.0.to_be_bytes());
			assert_eq!(base32_word, format!("0{}", repr));
			assert_eq!(base32::decode(&base32_word)?, keycode.0.to_be_bytes());
			let decoded: Keycode = repr.parse()?;
			assert_eq!(decoded, keycode, "(repr = {})", repr);
			assert_eq!(decoded.to_string(), repr);
		}
		Ok(())
	}
}
