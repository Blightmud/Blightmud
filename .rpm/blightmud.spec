%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: blightmud
Summary: A terminal mud-client
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: GPL3
Group: Applications/System
Source0: %{name}-%{version}.tar.gz
Source1: blightmud.d.lua
Source2: luarc.json

BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}
mkdir -p %{buildroot}%{_datadir}/blightmud/lua/types
install -m 644 %{SOURCE1} %{buildroot}%{_datadir}/blightmud/lua/types/blightmud.d.lua
install -m 644 %{SOURCE2} %{buildroot}%{_datadir}/blightmud/luarc.json

%clean
rm -rf %{buildroot}

%files
%defattr(-,root,root,-)
%{_bindir}/*
%{_datadir}/blightmud/
