/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Implementation of cookie storage as specified in
//! http://tools.ietf.org/html/rfc6265

use url::Url;
use cookie::Cookie;
use std::cmp::Ordering;

/// The creator of a given cookie
#[derive(PartialEq, Copy)]
pub enum CookieSource {
    /// An HTTP API
    HTTP,
    /// A non-HTTP API
    NonHTTP,
}

pub struct CookieStorage {
    cookies: Vec<Cookie>
}

impl CookieStorage {
    pub fn new() -> CookieStorage {
        CookieStorage {
            cookies: Vec::new()
        }
    }

    // http://tools.ietf.org/html/rfc6265#section-5.3
    pub fn remove(&mut self, cookie: &Cookie, source: CookieSource) -> Result<Option<Cookie>, ()> {
        // Step 1
        let position = self.cookies.iter().position(|c| {
            c.cookie.domain == cookie.cookie.domain &&
            c.cookie.path == cookie.cookie.path &&
            c.cookie.name == cookie.cookie.name
        });

        if let Some(ind) = position {
            let c = self.cookies.remove(ind);

            // http://tools.ietf.org/html/rfc6265#section-5.3 step 11.2
            if !c.cookie.httponly || source == CookieSource::HTTP {
                Ok(Some(c))
            } else {
                // Undo the removal.
                self.cookies.push(c);
                Err(())
            }
        } else {
            Ok(None)
        }
    }

    // http://tools.ietf.org/html/rfc6265#section-5.3
    pub fn push(&mut self, mut cookie: Cookie, source: CookieSource) {
        let old_cookie = self.remove(&cookie, source);
        if old_cookie.is_err() {
            // This new cookie is not allowed to overwrite an existing one.
            return;
        }

        if cookie.cookie.value.is_empty() {
            return;
        }

        // Step 11
        if let Some(old_cookie) = old_cookie.unwrap() {
            // Step 11.3
            cookie.creation_time = old_cookie.creation_time;
        }

        // Step 12
        self.cookies.push(cookie);
    }

    // http://tools.ietf.org/html/rfc6265#section-5.4
    pub fn cookies_for_url(&mut self, url: &Url, source: CookieSource) -> Option<String> {
        let filterer = |&:c: &&mut Cookie| -> bool {
            info!(" === SENT COOKIE : {} {} {:?} {:?}", c.cookie.name, c.cookie.value, c.cookie.domain, c.cookie.path);
            info!(" === SENT COOKIE RESULT {}", c.appropriate_for_url(url, source));
            // Step 1
            c.appropriate_for_url(url, source)
        };

        // Step 2
        let mut url_cookies: Vec<&mut Cookie> = self.cookies.iter_mut().filter(filterer).collect();
        url_cookies.sort_by(|a, b| {
            let a_path_len = a.cookie.path.as_ref().map(|p| p.len()).unwrap_or(0);
            let b_path_len = b.cookie.path.as_ref().map(|p| p.len()).unwrap_or(0);
            match a_path_len.cmp(&b_path_len) {
                Ordering::Equal => {
                    let a_creation_time = a.creation_time.to_timespec();
                    let b_creation_time = b.creation_time.to_timespec();
                    a_creation_time.cmp(&b_creation_time)
                }
                result => result
            }
        });

        let reducer = |&:acc: String, c: &mut &mut Cookie| -> String {
            // Step 3
            c.touch();

            // Step 4
            (match acc.len() {
                0 => acc,
                _ => acc + ";"
            }) + c.cookie.name.as_slice() + "=" + c.cookie.value.as_slice()
        };
        let result = url_cookies.iter_mut().fold("".to_string(), reducer);

        info!(" === COOKIES SENT: {}", result);
        match result.len() {
            0 => None,
            _ => Some(result)
        }
    }
}