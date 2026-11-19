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

[Code]
const
  LauncherStartUnconfirmed = 20;
  LauncherAbnormalExit = 21;

var
  BootstrapExitCode: Integer;

procedure InitializeWizard;
begin
  BootstrapExitCode := LauncherStartUnconfirmed;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  LauncherExitCode: Integer;
begin
  if CurStep <> ssPostInstall then
    Exit;

  try
    if not ExecAsOriginalUser(ExpandConstant('{app}\gui_shell_desktop_launcher.exe'),
      '', ExpandConstant('{app}'), SW_SHOWNORMAL, ewWaitUntilTerminated, LauncherExitCode) then
    begin
      Log('D4 Pocket 起動器を実行できません。Windows エラー=' + IntToStr(LauncherExitCode));
      Exit;
    end;
  except
    Log('D4 Pocket 起動器の実行を確認できません。');
    Exit;
  end;

  Log('D4 Pocket 起動器の終了コード=' + IntToStr(LauncherExitCode));
  if LauncherExitCode = 0 then
    BootstrapExitCode := 0
  else
    BootstrapExitCode := LauncherAbnormalExit;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID <> wpFinished then
    Exit;

  WizardForm.FinishedHeadingLabel.Caption := 'D4 Pocket セットアップの終了';
  case BootstrapExitCode of
    0:
      WizardForm.FinishedLabel.Caption :=
        'D4 Pocket の起動器が正常に終了しました。製品の導入状態はアプリ内の更新センターで確認できます。';
    LauncherStartUnconfirmed:
      WizardForm.FinishedLabel.Caption := 'セットアップから D4 Pocket の起動を確認できませんでした。';
    LauncherAbnormalExit:
      WizardForm.FinishedLabel.Caption :=
        'D4 Pocket が異常終了しました。起動時に表示されたエラーを確認してください。';
  end;
end;

function GetCustomSetupExitCode: Integer;
begin
  Result := BootstrapExitCode;
end;
