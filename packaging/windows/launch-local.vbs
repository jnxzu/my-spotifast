Option Explicit
Dim shell, files, quote, uri, pattern, command, code
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
quote = Chr(34)
uri = ""
If WScript.Arguments.Count > 1 Then WScript.Quit 2
If WScript.Arguments.Count = 1 Then
    uri = WScript.Arguments(0)
    Set pattern = New RegExp
    pattern.Pattern = "^spotify:[A-Za-z0-9:_%+./?=&,-]+$"
    If Len(uri) > 8192 Or Not pattern.Test(uri) Then WScript.Quit 2
End If
If files.FileExists("{SCRIPT}") And files.FileExists("{CONFIGURATION}") Then
    command = quote & "{POWERSHELL}" & quote & " -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File " & _
        quote & "{SCRIPT}" & quote & " -Configuration " & quote & "{CONFIGURATION}" & quote
    If uri <> "" Then command = command & " -OpenUri " & quote & uri & quote
    code = shell.Run(command, 0, True)
    If code = 0 Then WScript.Quit 0
End If
command = quote & "{EXECUTABLE}" & quote
If uri <> "" Then command = command & " " & quote & uri & quote
shell.Run command, 1, False
