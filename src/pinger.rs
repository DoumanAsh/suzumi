use std::net::IpAddr;
use std::{io, time};

pub struct Pinger {
    addr: IpAddr,
}

impl Pinger {
    ///Creates new instance
    ///
    ///`addr` - Destination address for the ICMP packet.
    pub const fn new(addr: IpAddr) -> Self {
        Self {
            addr,
        }
    }

    #[inline]
    pub fn ping(&self) -> io::Result<time::Duration> {
        self.ping_timeout(time::Duration::from_millis(250))
    }

    pub fn ping_timeout(&self, timeout: time::Duration) -> io::Result<time::Duration> {
        const PING_FAIL_ERROR: &str = "Ping command cannot reach host";
        const PING_FAIL_OUTPUT: &str = "Unable to parse ping output";
        let mut cmd = std::process::Command::new("ping");

        #[cfg(unix)]
        cmd.arg("-c");
        #[cfg(windows)]
        cmd.arg("-n");
        cmd.arg("1");

        #[cfg(unix)]
        cmd.arg("-W");
        #[cfg(windows)]
        cmd.arg("-w");
        match timeout.as_secs() {
            0 | 1 => cmd.arg("1"),
            secs => cmd.arg(format!("{}", secs)),
        };

        cmd.arg(format!("{}", self.addr));
        let output = cmd.output()?;

        if !output.status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, PING_FAIL_ERROR));
        }

        let stdout = match core::str::from_utf8(&output.stdout) {
            Ok(stdout) => stdout,
            Err(_) => return Ok(time::Duration::from_secs(0)),
        };

        for line in stdout.lines() {
            const TIME_PREFIX: &str = "time=";
            if let Some(time_pos) = line.find(TIME_PREFIX) {
                let line = match line.get(time_pos + TIME_PREFIX.len()..) {
                    Some(line) => line,
                    None => break,
                };

                if let Some(ms_pos) = line.find("ms") {
                    let line = match line.get(..ms_pos) {
                        Some(line) => line.trim(),
                        None => break,
                    };

                    let secs = match line.parse::<f64>() {
                        Ok(ms) => ms / 1000.0f64,
                        Err(_) => 0.0f64,
                    };
                    return Ok(time::Duration::from_secs_f64(secs));
                } else {
                    break;
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::InvalidData, PING_FAIL_OUTPUT))
    }
}

#[cfg(test)]
mod tests {
    use super::Pinger;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn pinger_should_ping_google_dns() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let pinger = Pinger::new(ip);

        let duration = pinger.ping().expect("To ping");
        assert_ne!(duration.as_millis(), 0);
    }

    #[test]
    fn pinger_should_ping_non_existing() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 9));
        let pinger = Pinger::new(ip);

        let duration = pinger.ping().expect_err("Fail to ping");
    }
}
