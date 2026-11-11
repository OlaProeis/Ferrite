; Ferrite MD Portable - PortableApps.com Format Installer
; This script creates a .paf.exe self-extracting installer that follows
; the PortableApps.com Format specification.
;
; Build:
;   makensis /DAPPVERSION=0.3.1 installer.nsi
;
; If APPVERSION is not passed, defaults to "0.0.0".

Unicode true

!include "MUI2.nsh"
!include "FileFunc.nsh"

; --- Application metadata ---
!define APPNAME "Ferrite MD Portable"
!define APPID "FerriteMDPortable"
!ifndef APPVERSION
  !define APPVERSION "0.0.0"
!endif
!define APPPUBLISHER "OlaProeis"
!define APPHOMEPAGE "https://github.com/OlaProeis/Ferrite"

; --- Installer settings ---
Name "${APPNAME} ${APPVERSION}"
OutFile "${APPID}_${APPVERSION}_English.paf.exe"
InstallDir "\${APPID}"
ShowInstDetails nevershow

; --- Compression ---
SetCompressor /SOLID lzma
SetCompressorDictSize 32

; --- MUI settings ---
!define MUI_ICON "FerriteMDPortable\App\AppInfo\appicon.ico"
!define MUI_ABORTWARNING

; --- Directory page ---
!define MUI_DIRECTORYPAGE_TEXT_TOP "Setup will install ${APPNAME} ${APPVERSION} in the following folder. To install in a different folder, click Browse and select another folder. Click Install to start the installation."
!define MUI_DIRECTORYPAGE_TEXT_DESTINATION "Destination Folder"

; --- Pages ---
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

; --- Language ---
!insertmacro MUI_LANGUAGE "English"

; --- Install section ---
Section "Install"
    SetOutPath "$INSTDIR"

    ; Copy the entire FerriteMDPortable directory structure
    File "FerriteMDPortable\FerriteMDPortable.exe"
    File "FerriteMDPortable\help.html"

    SetOutPath "$INSTDIR\App\AppInfo"
    File "FerriteMDPortable\App\AppInfo\appinfo.ini"
    File "FerriteMDPortable\App\AppInfo\appicon.ico"
    File "FerriteMDPortable\App\AppInfo\appicon_16.png"
    File "FerriteMDPortable\App\AppInfo\appicon_32.png"
    File "FerriteMDPortable\App\AppInfo\appicon_128.png"

    SetOutPath "$INSTDIR\App\AppInfo\Launcher"
    File "FerriteMDPortable\App\AppInfo\Launcher\FerriteMDPortable.ini"

    SetOutPath "$INSTDIR\App\Ferrite"
    File "FerriteMDPortable\App\Ferrite\ferrite.exe"

    SetOutPath "$INSTDIR\App\DefaultData\settings"

    SetOutPath "$INSTDIR\Data\settings"

    SetOutPath "$INSTDIR\Other\Source"
    File "FerriteMDPortable\Other\Source\LICENSE"

    SetOutPath "$INSTDIR\Other\Help"
    File "FerriteMDPortable\Other\Help\help.html"

    CreateDirectory "$INSTDIR\Other\Help\Images"
SectionEnd
