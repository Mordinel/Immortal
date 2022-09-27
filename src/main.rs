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

use crate::immortal::Immortal;
mod immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let mut immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };

    immortal.route.register("GET", "/", |_req, res| {
        res.code = "200".to_string();
        res.body = b"<h1>200: Ok</h1>".to_vec();
    });

    immortal.route.register("GET", "/favicon.ico", |_req, res| {
        res.code = "404".to_string();
        res.body = b"<h1>404: Not found</h1>".to_vec();
    });

    immortal.route.register("GET", "/teapot", |_req, res| {
        res.code = "418".to_string();
        res.body = b"<h1>418: I am a little teapot</h1>".to_vec();
    });


    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
