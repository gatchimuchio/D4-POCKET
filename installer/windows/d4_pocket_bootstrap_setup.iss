#ifndef BundleDir
  #error BundleDir must point to a validated Windows Export bundle
#endif
#ifndef ProductAppId
  #error ProductAppId must be read from the validated Product Manifest
#endif
#ifndef ProductVersion
  #error ProductVersion must match the current source version
#endif

[Setup]
AppId=D4Pocket-{#ProductAppId}
AppName=D4 Pocket
AppVersion={#ProductVersion}
DefaultDirName={tmp}\D4PocketBootstrap-{#ProductAppId}
PrivilegesRequired=lowest
Uninstallable=no
CreateUninstallRegKey=no
CreateAppDir=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
CloseApplications=no
RestartIfNeededByRun=no
OutputBaseFilename=D4Pocket-Setup-{#ProductVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Files]
Source: "{#BundleDir}\app\*"; DestDir: "{app}\app"; Flags: recursesubdirs createallsubdirs
Source: "{#BundleDir}\broker\gui_shell_rust_helper.exe"; DestDir: "{app}\broker"
Source: "{#BundleDir}\gui_shell_desktop_launcher.exe"; DestDir: "{app}"
Source: "{#BundleDir}\product_manifest.json"; DestDir: "{app}"

[Run]
Filename: "{app}\gui_shell_desktop_launcher.exe"; WorkingDir: "{app}"; Description: "D4 Pocketを起動"; Flags: waituntilterminated runasoriginaluser
