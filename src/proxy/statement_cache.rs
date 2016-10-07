// Copyright 2016 AgilData
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http:// www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Mutex;
use std::rc::Rc;
use std::collections::HashMap;
use query::{Token};

use super::physical_planner::{PhysicalPlan};

pub struct StatementCache {
    cache: Mutex<HashMap<Vec<Token>, Rc<PhysicalPlan>>>
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: Mutex::new(HashMap::new())
        }
    }

    pub fn get(&self, key: &Vec<Token>) -> Option<Rc<PhysicalPlan>> {
        let data = self.cache.lock().unwrap();

        match data.get(key) {
            Some(rc) => Some(rc.clone()),
            None => None
        }
    }

    pub fn put(&self, key: Vec<Token>, ep: PhysicalPlan) -> Rc<PhysicalPlan> {
        let mut data = self.cache.lock().unwrap();
        let value = Rc::new(ep);
        let reference = value.clone();
        data.insert(key, value);
        reference
    }
}