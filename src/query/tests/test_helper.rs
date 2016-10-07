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

pub fn format_sql(sql: &str) -> String {

    sql.to_uppercase()

        // unify datatype synonymns
        .replace("BOOLEAN", "BOOL").replace("BOOL", "BOOLEAN") // done twice intentionally
        .replace("INTEGER", "INT").replace(" INT", " INTEGER")
        .replace("PRECISION", "")
        .replace("DECIMAL", "DEC").replace("DEC", "DECIMAL")
        .replace("CHARACTER VARYING", "VARCHAR")
        .replace("NATIONAL CHARACTER", "NCHAR")
        .replace("NATIONAL CHAR", "NCHAR")
        .replace("NATIONAL VARCHAR", "NVARCHAR")
        .replace("CHARACTER", "CHAR")

        // optional keywords
        .replace("ASC", "")
        .replace("INNER JOIN", "JOIN")

        // strip whitespace
        .replace(" ", "").replace("\n", "").replace("\r", "").replace("\t", "")


}
