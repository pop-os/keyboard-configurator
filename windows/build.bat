SET PATH=C:\msys64\mingw32\bin;%PATH%;C:\msys64\usr\bin
rustup run stable-i686-pc-windows-gnu cargo build --release --examples
mkdir out
xcopy ..\target\release\examples\keyboard_color.exe out\ /Y
strip out\keyboard_color.exe
xcopy C:\msys64\mingw32\bin\libcairo-gobject-2.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libcairo-2.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgdk-3-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgdk_pixbuf-2.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgio-2.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgobject-2.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libglib-2.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgtk-3-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpango-1.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgcc_s_seh-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgcc_s_dw2-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libssp-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libfontconfig-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libfreetype-6.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpixman-1-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpng16-16.dll out\ /Y
xcopy C:\msys64\mingw32\bin\zlib1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libepoxy-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libfribidi-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libintl-8.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgmodule-2.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libffi-7.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpangocairo-1.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpangowin32-1.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libwinpthread-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpcre-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libatk-1.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libharfbuzz-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libthai-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libpangoft2-1.0-0.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libexpat-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libiconv-2.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libbrotlidec.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libbz2-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libgraphite2.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libdatrie-1.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libbrotlicommon.dll out\ /Y
xcopy C:\msys64\mingw32\bin\libstdc++-6.dll out\ /Y
