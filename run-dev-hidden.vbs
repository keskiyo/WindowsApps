Set shell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
root = fso.GetParentFolderName(WScript.ScriptFullName)
quote = Chr(34)
primaryExe = root & "\src-tauri\target\release\app.exe"
alternateExe = root & "\.cargo-target\release\app.exe"
exe = ""
If fso.FileExists(primaryExe) Then exe = primaryExe
If fso.FileExists(alternateExe) Then
  If exe = "" Then
    exe = alternateExe
  ElseIf fso.GetFile(alternateExe).DateLastModified > fso.GetFile(primaryExe).DateLastModified Then
    exe = alternateExe
  End If
End If
If exe <> "" Then
  shell.Run quote & exe & quote, 1, False
Else
  MsgBox "Windows Apps is not built yet.", 48, "Windows Apps"
End If
