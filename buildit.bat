@echo off
set PATH=%PATH:C:\msys64\ucrt64\bin;=%
set PATH=%PATH:C:\msys64\mingw64\bin;=%
set PATH=%PATH:C:\msys64\usr\bin;=%


:: 1. Point to FFmpeg
set FFMPEG_DIR=C:\ffmpeg
set FFMPEG_INCLUDE_DIR=C:\ffmpeg\include

:: 2. Fix the missing C++ Runtime headers (Note the new VC path at the end)
set INCLUDE=C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared;C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.50.35717\include;%INCLUDE%

:: 3. Fix the Libraries
set LIB=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.50.35717\lib\x64;%LIB%

::cargo clean
cargo build --release

if %errorlevel% LEQ 0 (
robocopy ./target/release/ ./ "beforefx.exe"
)