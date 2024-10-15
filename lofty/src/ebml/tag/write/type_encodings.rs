use super::{EbmlWriteExt, ElementWriterCtx};
use crate::ebml::{TagValue, VInt};
use crate::error::Result;
use std::io::Write;

use byteorder::WriteBytesExt;

pub(crate) trait ElementEncodable {
	fn len(&self) -> Result<VInt<u64>>;

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()>;
}

impl ElementEncodable for VInt<u64> {
	fn len(&self) -> Result<VInt<u64>> {
		Ok(VInt(u64::from(self.octet_length())))
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		writer.write_size(ctx, self.len()?)?;
		VInt::<u64>::write_to(self.value(), None, None, writer)?;
		Ok(())
	}
}

impl ElementEncodable for VInt<i64> {
	fn len(&self) -> Result<VInt<u64>> {
		Ok(VInt(u64::from(self.octet_length())))
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		writer.write_size(ctx, self.len()?)?;
		VInt::<i64>::write_to(self.value() as u64, None, None, writer)?;
		Ok(())
	}
}

impl ElementEncodable for f32 {
	fn len(&self) -> Result<VInt<u64>> {
		Ok(VInt(size_of::<f32>() as u64))
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		if *self == 0.0 {
			VInt::<u64>::write_to(VInt::<u64>::ZERO.value(), None, None, writer)?;
			return Ok(());
		}

		writer.write_size(ctx, self.len()?)?;
		writer.write_f32::<byteorder::BigEndian>(*self)?;
		Ok(())
	}
}

impl ElementEncodable for f64 {
	fn len(&self) -> Result<VInt<u64>> {
		Ok(VInt(size_of::<f64>() as u64))
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		if *self == 0.0 {
			VInt::<u64>::write_to(VInt::<u64>::ZERO.value(), None, None, writer)?;
			return Ok(());
		}

		writer.write_size(ctx, self.len()?)?;
		writer.write_f64::<byteorder::BigEndian>(*self)?;
		Ok(())
	}
}

impl ElementEncodable for bool {
	fn len(&self) -> Result<VInt<u64>> {
		Ok(VInt(size_of::<bool>() as u64))
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		match *self {
			true => VInt::<u64>(1).write_to(ctx, writer),
			false => VInt::<i64>::ZERO.write_to(ctx, writer),
		}
	}
}

impl ElementEncodable for &[u8] {
	fn len(&self) -> Result<VInt<u64>> {
		VInt::try_from(<[u8]>::len(self) as u64)
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		writer.write_size(ctx, <&[u8] as ElementEncodable>::len(self)?)?;
		writer.write_all(self)?;
		Ok(())
	}
}

impl ElementEncodable for &str {
	fn len(&self) -> Result<VInt<u64>> {
		VInt::try_from(str::len(self) as u64)
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		writer.write_size(ctx, <&str as ElementEncodable>::len(self)?)?;
		writer.write_all(self.as_bytes())?;
		Ok(())
	}
}

impl ElementEncodable for TagValue<'_> {
	fn len(&self) -> Result<VInt<u64>> {
		match self {
			TagValue::String(s) => <&str as ElementEncodable>::len(&&**s),
			TagValue::Binary(b) => <&[u8] as ElementEncodable>::len(&&**b),
		}
	}

	fn write_to<W: Write>(&self, ctx: ElementWriterCtx, writer: &mut W) -> Result<()> {
		match self {
			TagValue::String(s) => <&str as ElementEncodable>::write_to(&&**s, ctx, writer),
			TagValue::Binary(b) => <&[u8] as ElementEncodable>::write_to(&&**b, ctx, writer),
		}
	}
}
