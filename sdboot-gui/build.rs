#[cfg(target_os = "windows")]
fn main() {
    let mut resource = winres::WindowsResource::new();
    resource.set_manifest(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#,
    );
    if let Err(error) = resource.compile() {
        eprint!("{}", error);
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
