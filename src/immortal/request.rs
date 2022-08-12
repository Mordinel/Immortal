/**
*     Copyright (C) 2022 Mason Soroka-Gill
*
*     This program is free software: you can redistribute it and/or modify
*     it under the terms of the GNU General Public License as published by
*     the Free Software Foundation, either version 3 of the License, or
*     (at your option) any later version.
*
*     This program is distributed in the hope that it will be useful,
*     but WITHOUT ANY WARRANTY; without even the implied warranty of
*     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*     GNU General Public License for more details.
*
*     You should have received a copy of the GNU General Public License
*     along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

pub struct Request {
    //method: String,
    //document: String,
    //query: String,
    //version: String,
    //headers: Vec<String>,
    
    //Future header fields to be parsed
    //host: String
    //user_agent: String,
    //connection: String,
    //content_type: String,
    //content_length: String,
    //cookies_raw: String,
    //keep_alive: bool,
}

impl Request {
    /**
     * Construct a new request object, parsing the request buffer
     */
    pub fn new(buf: &mut [u8]) -> Result<Self, String> {
        let (request_head, request_body) = match request_head_body_split(buf) {
            Err(e) => return Err(e),
            Ok(tuple) => tuple,
        };
        println!("{}\n\n{}", String::from_utf8_lossy(&request_head), String::from_utf8_lossy(&request_body.unwrap()));
        Err("Not implemented".to_string())
    }
}

/**
 * Find the index of the first double crlf and return a tuple of two mutable string slices, the
 * first being the buffer content up to the double crlf, and the second being being the buffer
 * content after the double clrf
 */
fn request_head_body_split(haystack: &mut [u8]) -> Result<(&mut [u8], Option<&mut [u8]>), String>  {
    let mut found_cr = false;
    let mut crlf_count = 0;
    let mut crlf_start_idx = 0;

    for (idx, byte) in haystack.iter().enumerate() {
        if crlf_count == 2 { // exit case where crlf_start_index can be not zero
            break;
        }
        if *byte == ('\r' as u8) {
            if crlf_count == 0 { // record the crlf start index only on the first crlf
                crlf_start_idx = idx;
            }
            found_cr = true;
            continue;
        }
        if found_cr && *byte == ('\n' as u8) {
            crlf_count += 1;
            found_cr = false;
            continue;
        }
        crlf_count = 0;
        crlf_start_idx = 0;
        found_cr = false;
    }

    if crlf_start_idx == 0 {
        return Ok((haystack, None));
    }

    let (mut head, mut body) = haystack.split_at_mut(crlf_start_idx);
    body = body.split_at_mut(4).1;
    Ok((head, Some(body)))
}
