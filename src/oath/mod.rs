//
// Copyright (c) 2015 Rodolphe Breard
// 
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
// 
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//

#[repr(C)]
#[derive(Clone, Copy)]
pub enum HashFunction {
    Sha1 = 1,
    Sha256 = 2,
    Sha512 = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum ErrorCode {
    CfgNullPtr      = 1,
    CodeNullPtr     = 2,
    KeyNullPtr      = 3,

    InvalidBaseLen  = 10,
    InvalidKeyLen   = 11,
    CodeTooSmall    = 12,
    CodeTooBig      = 13,

    InvalidKey      = 20,
    InvalidPeriod   = 21,

    CodeInvalidUTF8 = 30,
}


macro_rules! builder_common {
    ($t:ty) => {
        /// Sets the shared secret.
        pub fn key(&mut self, key: &Vec<u8>) -> &mut $t {
            self.key = Some(key.clone());
            self
        }

        /// Sets the shared secret. This secret is passed as an ASCII string.
        pub fn ascii_key(&mut self, key: &String) -> &mut $t {
            self.key = Some(key.clone().into_bytes());
            self
        }

        /// Sets the shared secret. This secret is passed as an hexadecimal encoded string.
        pub fn hex_key(&mut self, key: &String) -> &mut $t {
            match key.from_hex() {
                Ok(k) => { self.key = Some(k); }
                Err(_) => { self.runtime_error = Some(ErrorCode::InvalidKey); }
            }
            self
        }

        /// Sets the shared secret. This secret is passed as a base32 encoded string.
        pub fn base32_key(&mut self, key: &String) -> &mut $t {
            match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &key) {
                Some(k) => { self.key = Some(k); }
                None => { self.runtime_error = Some(ErrorCode::InvalidKey); }
            }
            self
        }

        fn code_length(&self) -> usize {
            let base_len = self.output_base.len();
            let mut nb_bits = base_len;
            for _ in 1..self.output_len {
                nb_bits = match nb_bits.checked_mul(base_len) {
                    Some(nb_bits) => nb_bits,
                    None => return ::std::usize::MAX,
                };
            }
            nb_bits
        }

        /// Sets the number of characters for the code. The minimum and maximum values depends the base. Default is 6.
        pub fn output_len(&mut self, output_len: usize) -> &mut $t {
            self.output_len = output_len;
            self
        }

        /// Sets the base used to represents the output code. Default is "0123456789".to_owned().into_bytes().
        pub fn output_base(&mut self, base: &Vec<u8>) -> &mut $t {
            self.output_base = base.clone();
            self
        }

        /// Sets the hash function. Default is Sha1.
        pub fn hash_function(&mut self, hash_function: HashFunction) -> &mut $t {
            self.hash_function = hash_function;
            self
        }
    }
}

#[cfg(feature = "cbindings")]
pub mod c {
    use super::ErrorCode;
    use std;

    pub fn write_code(code: &Vec<u8>, dest: &mut [u8]) {
        let len = code.len();
        for i in 0..len {
            dest[i] = code[i];
        };
        dest[len] = 0;
    }

    pub fn get_cfg<T>(cfg: *const T) -> Result<&'static T, ErrorCode> {
        if cfg.is_null() {
            return Err(ErrorCode::CfgNullPtr)
        }
        let cfg: &T = unsafe { &*cfg };
        Ok(cfg)
    }

    pub fn get_code(code: *const u8, code_len: usize) -> Result<String, ErrorCode> {
        if code.is_null() {
            return Err(ErrorCode::CodeNullPtr)
        }
        let code = unsafe { std::slice::from_raw_parts(code, code_len).to_owned() };
        match String::from_utf8(code) {
            Ok(code) => Ok(code),
            Err(_) => Err(ErrorCode::CodeInvalidUTF8),
        }
    }

    pub fn get_mut_code(code: *mut u8, code_len: usize) -> Result<&'static mut [u8], ErrorCode> {
        if code.is_null() {
            return Err(ErrorCode::CodeNullPtr)
        }
        Ok(unsafe { std::slice::from_raw_parts_mut(code, code_len + 1) })
    }

    pub fn get_output_base(output_base: *const u8, output_base_len: usize) -> Result<Vec<u8>, ErrorCode> {
        match output_base.is_null() {
            false => {
                match output_base_len {
                    0 | 1 => Err(ErrorCode::InvalidBaseLen),
                    l => Ok(unsafe { std::slice::from_raw_parts(output_base, l).to_owned() })
                }
            },
            true => Ok("0123456789".to_owned().into_bytes()),
        }
    }

    pub fn get_key(key: *const u8, key_len: usize) -> Result<Vec<u8>, ErrorCode> {
        match key.is_null() {
            false => {
                match key_len {
                    0 => Err(ErrorCode::InvalidKeyLen),
                    l => Ok(unsafe { std::slice::from_raw_parts(key, l).to_owned() }),
                }
            },
            true => Err(ErrorCode::KeyNullPtr),
        }
    }
}

#[cfg(feature = "cbindings")]
macro_rules! otp_init {
    ($cfg_type:ty, $cfg:ident, $($field:ident, $value:expr), *) => {
        match $cfg.is_null() {
            false => {
                let c: &mut $cfg_type = unsafe { &mut *$cfg };
                c.key = std::ptr::null();
                c.key_len = 0;
                c.output_len = 6;
                c.output_base = std::ptr::null();
                c.output_base_len = 0;
                c.hash_function = HashFunction::Sha1;
                $(
                    c.$field = $value;
                )*
                Ok(c)
            }
            true => Err(ErrorCode::CfgNullPtr),
        }
    }
}

#[cfg(feature = "cbindings")]
macro_rules! get_value_or_errno {
    ($val:expr) => {{
        match $val {
            Ok(v) => v,
            Err(errno) => return errno as libc::int32_t,
        }
    }}
}

#[cfg(feature = "cbindings")]
macro_rules! get_value_or_false {
    ($val:expr) => {{
        match $val {
            Ok(v) => v,
            Err(_) => return 0,
        }
    }}
}


pub mod hotp;
pub mod totp;

pub type HOTPBuilder = hotp::HOTPBuilder;
pub type TOTPBuilder = totp::TOTPBuilder;