//! Encode: pipes raw RGBA frames into an ffmpeg subprocess and muxes in a WAV.

use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("could not start ffmpeg (is it on PATH?): {0}")]
    Spawn(io::Error),
    #[error("writing frames to ffmpeg failed: {0}")]
    Pipe(io::Error),
    #[error("ffmpeg failed ({status}): {stderr}")]
    Ffmpeg { status: String, stderr: String },
}

pub struct VideoEncoder {
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
}

impl VideoEncoder {
    /// Starts ffmpeg writing `out` (H.264 + AAC .mp4) from `fps` frames of
    /// `width`x`height` RGBA on stdin plus the audio in `wav`.
    pub fn start(
        out: &Path,
        wav: &Path,
        width: u32,
        height: u32,
        fps: u64,
    ) -> Result<Self, EncodeError> {
        let mut child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba"])
            .args(["-s", &format!("{width}x{height}")])
            .args(["-r", &fps.to_string()])
            .args(["-i", "-"])
            .arg("-i")
            .arg(wav)
            .args(["-map", "0:v", "-map", "1:a"])
            .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18"])
            .args(["-pix_fmt", "yuv420p", "-movflags", "+faststart"])
            .args(["-c:a", "aac", "-b:a", "192k"])
            .arg("-shortest")
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(EncodeError::Spawn)?;
        let stdin = child
            .stdin
            .take()
            .map(|s| BufWriter::with_capacity(1 << 20, s));
        Ok(VideoEncoder { child, stdin })
    }

    pub fn write_frame(&mut self, rgba: &[u8]) -> Result<(), EncodeError> {
        let stdin = self.stdin.as_mut().expect("encoder already finished");
        stdin.write_all(rgba).map_err(EncodeError::Pipe)
    }

    /// Closes the frame pipe and waits for ffmpeg to finish writing the file.
    pub fn finish(mut self) -> Result<(), EncodeError> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin.flush().map_err(EncodeError::Pipe)?;
        }
        let output = self.child.wait_with_output().map_err(EncodeError::Spawn)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(EncodeError::Ffmpeg {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }
}
