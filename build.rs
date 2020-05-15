fn main()
{
	println!("cargo:rustc-flags=-l dylib=stdc++");
	println!("cargo:rustc-link-search=native=bin/");
	println!("cargo:rustc-link-lib=static=epicinium");
}
