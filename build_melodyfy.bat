@echo off
echo Compilando Melodyfy...
"C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe" /target:winexe /out:Melodyfy.exe /reference:System.Windows.Forms.dll,System.Drawing.dll,System.Web.Extensions.dll Melodyfy.cs
if %errorlevel% == 0 (
    echo Compilacion exitosa.
) else (
    echo Error en la compilacion.
)
