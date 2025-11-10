use std::fmt::Display;

#[derive(Debug)]
pub enum MpegFrameError {
	BadVersion,
	BadLayer,
	BadBitrate,
	BadSampleRate,
}

impl From<MpegFrameError> for crate::error::AudioError {
	fn from(err: MpegFrameError) -> Self {
		crate::error::AudioError::Mpeg(err.into())
	}
}

impl Display for MpegFrameError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			MpegFrameError::BadVersion => write!(f, "Invalid MPEG frame version"),
			MpegFrameError::BadLayer => write!(f, "Invalid MPEG frame layer"),
			MpegFrameError::BadBitrate => write!(f, "MPEG frame has an invalid bitrate index"),
			MpegFrameError::BadSampleRate => write!(f, "MPEG frame has an sample rate index"),
		}
	}
}

impl core::error::Error for MpegFrameError {}

#[derive(Debug)]
pub enum VbrHeaderError {
	BadXing,
	BadVbri,
	UnknownHeader,

	Io(std::io::Error),
}

impl From<std::io::Error> for VbrHeaderError {
	fn from(err: std::io::Error) -> Self {
		VbrHeaderError::Io(err)
	}
}

impl From<VbrHeaderError> for crate::error::AudioError {
	fn from(err: VbrHeaderError) -> Self {
		crate::error::AudioError::Mpeg(err.into())
	}
}

impl Display for VbrHeaderError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			VbrHeaderError::BadXing => write!(f, "Xing header is invalid"),
			VbrHeaderError::BadVbri => write!(f, "VBRI header is invalid"),
			VbrHeaderError::UnknownHeader => write!(f, "Unknown VBR header type"),
			VbrHeaderError::Io(e) => write!(f, "{e}"),
		}
	}
}

impl core::error::Error for VbrHeaderError {}

#[derive(Debug)]
pub enum MpegError {
	Frame(MpegFrameError),
	Vbr(VbrHeaderError),
}

impl From<MpegFrameError> for MpegError {
	fn from(err: MpegFrameError) -> Self {
		MpegError::Frame(err)
	}
}

impl From<VbrHeaderError> for MpegError {
	fn from(err: VbrHeaderError) -> Self {
		MpegError::Vbr(err)
	}
}

impl Display for MpegError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			MpegError::Frame(err) => write!(f, "{err}"),
			MpegError::Vbr(err) => write!(f, "{err}"),
		}
	}
}

impl core::error::Error for MpegError {}
