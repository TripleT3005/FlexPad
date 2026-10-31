%global debug_package %{nil}

Name: flexpad
Version: APP_VERSION
Release: 1
Summary: Configurator for the FlexPad macro pad
License: MIT
URL: https://github.com/TripleT3005/FlexPad

%description
Desktop app to place buttons and rotary encoders of an ESP32-S3 based
macro pad on a canvas, assign GPIO pins and macros, and send the
configuration to the device over USB serial.

%install
cp -a %{_sourcedir}/root/. %{buildroot}/

%files
%{_bindir}/flexpad
%{_datadir}/applications/flexpad.desktop
%{_datadir}/icons/hicolor/*/apps/flexpad.*
%{_docdir}/flexpad
