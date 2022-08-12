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

    let immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{}", e),
        Ok(i) => i,
    };
    if let Err(e) = immortal.listen() {
        panic!("{}", e);
    }
}
