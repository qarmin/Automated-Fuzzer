class AFT_FSDK_Version(Structure):
    _fields_ = [('lCodebase',c_int32),('lMajor',c_int32),('lMinor',c_int32),('lBuild',c_int32),
               ('Version',c_har_p),('BuildDate',c_char_p),('CopyRight',c_char_p)]
AFT_FSDK_OPF_90_ONLY = 0x2;       90; 90; ...