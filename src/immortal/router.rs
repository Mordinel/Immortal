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

use std::collections::HashMap;
use crate::immortal::request::Request;
use crate::immortal::response::Response;

pub type Handler = fn(&Request, &mut Response);

pub struct Router {
    fallback: Handler,
    routes: HashMap<String, HashMap<String, Handler>>,
}

fn not_implemented(_req: &Request, res: &mut Response) {
    res.code = "501".to_string();
    res.status = "NOT IMPLEMENTED".to_string();
    res.body = b"<h1>501: Not Implemented</h1>".to_vec();
}

impl Router {
    pub fn new() -> Self {
        Self {
            fallback: not_implemented,
            routes: HashMap::new(),
        }
    }

    pub fn register(&mut self, method: &str, route: &str, func: Handler) -> bool {
        self.routes.insert(method.to_string(), HashMap::new());
        match self.routes.get_mut(method) {
            None => return false,
            Some(inner) => inner,
        }.insert(route.to_string(), func);
        return true;
    }

    pub fn call(&self, method: &str, req: &Request, res: &mut Response) {
        let by_method = match self.routes.get(method) {
            None => {
                (self.fallback)(req, res);
                return;
            },
            Some(inner) => inner,
        };
        let func = match by_method.get(&req.document) {
            None => {
                (self.fallback)(req, res);
                return;
            },
            Some(inner) => inner,
        };
        func(req, res);
    }
}

