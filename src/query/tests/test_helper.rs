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
