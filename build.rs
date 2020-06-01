fn main()
{
	println!("cargo:rerun-if-changed=bin/libepicinium.a");
	println!("cargo:rustc-flags=-l dylib=stdc++");
	println!("cargo:rustc-link-search=native=bin/");
	println!("cargo:rustc-link-lib=static=epicinium");
}
