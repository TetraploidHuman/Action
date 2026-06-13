; ModuleID = 'main'
source_filename = "main"

%__action_str = type { i64, ptr }

@.fmt_int = global [4 x i8] c"%ld\00"
@.fmt_float = global [3 x i8] c"%g\00"
@.fmt_str = global [3 x i8] c"%s\00"
@.fmt_nl = global [2 x i8] c"\0A\00"
@.str_true = global [5 x i8] c"true\00"
@.str_false = global [6 x i8] c"false\00"
@.fmt_lb = global [2 x i8] c"[\00"
@.fmt_sep = global [3 x i8] c", \00"
@.fmt_rb = global [2 x i8] c"]\00"
@.fmt_task_pre = global [11 x i8] c"Task(done=\00"
@.fmt_task_mid = global [13 x i8] c", cancelled=\00"
@.fmt_task_suf = global [2 x i8] c")\00"
@.fmt_struct = global [9 x i8] c"<struct>\00"
@.fmt_ev_pre = global [13 x i8] c"EnumVariant<\00"
@.fmt_ev_gt = global [2 x i8] c">\00"
@.fmt_ev_lp = global [3 x i8] c">(\00"
@.fmt_ev_rp = global [2 x i8] c")\00"
@.fmt_int_str = global [4 x i8] c"%ld\00"
@.fmt_float_str = global [3 x i8] c"%g\00"
@.rf_mode = global [3 x i8] c"rb\00"
@.wf_mode = global [3 x i8] c"wb\00"
@.fe_mode = global [2 x i8] c"r\00"
@.fa_mode = global [2 x i8] c"a\00"
@action_rand_seed = global i64 123456789

declare i32 @printf(ptr, ...)

declare ptr @malloc(i64)

declare ptr @realloc(ptr, i64)

declare void @free(ptr)

define ptr @action_malloc_rc(i64 %0) {
entry:
  %total = add i64 %0, 8
  %raw = call ptr @malloc(i64 %total)
  store i64 0, ptr %raw, align 4
  %data = getelementptr i8, ptr %raw, i64 8
  ret ptr %data
}

declare i32 @memcmp(ptr, ptr, i64)

define i64 @action_utf8_encode(i64 %0, ptr %1) {
entry:
  %is1 = icmp ule i64 %0, 127
  br i1 %is1, label %one_byte, label %two_byte

one_byte:                                         ; preds = %entry
  %u1 = trunc i64 %0 to i8
  store i8 %u1, ptr %1, align 1
  ret i64 1

two_byte:                                         ; preds = %entry
  %is2 = icmp ule i64 %0, 2047
  br i1 %is2, label %three_byte, label %four_byte

three_byte:                                       ; preds = %two_byte
  %cp6 = lshr i64 %0, 6
  %l2t = trunc i64 %cp6 to i8
  %lead2 = or i8 %l2t, -64
  store i8 %lead2, ptr %1, align 1
  %cont2 = and i64 %0, 63
  %c2t = trunc i64 %cont2 to i8
  %b2 = or i8 %c2t, -128
  %gp1 = getelementptr i8, ptr %1, i64 1
  store i8 %b2, ptr %gp1, align 1
  ret i64 2

four_byte:                                        ; preds = %two_byte
  %is3 = icmp ule i64 %0, 65535
  br i1 %is3, label %three_byte_write, label %four_byte_write

three_byte_write:                                 ; preds = %four_byte
  %cp12 = lshr i64 %0, 12
  %l3t = trunc i64 %cp12 to i8
  %lead3 = or i8 %l3t, -32
  store i8 %lead3, ptr %1, align 1
  %cp6b = lshr i64 %0, 6
  %c3_1 = and i64 %cp6b, 63
  %c3_1t = trunc i64 %c3_1 to i8
  %b3_1 = or i8 %c3_1t, -128
  %gp3_1 = getelementptr i8, ptr %1, i64 1
  store i8 %b3_1, ptr %gp3_1, align 1
  %c3_2 = and i64 %0, 63
  %c3_2t = trunc i64 %c3_2 to i8
  %b3_2 = or i8 %c3_2t, -128
  %gp3_2 = getelementptr i8, ptr %1, i64 2
  store i8 %b3_2, ptr %gp3_2, align 1
  ret i64 3

four_byte_write:                                  ; preds = %four_byte
  %cp18 = lshr i64 %0, 18
  %l4t = trunc i64 %cp18 to i8
  %lead4 = or i8 %l4t, -16
  store i8 %lead4, ptr %1, align 1
  %cp12b4 = lshr i64 %0, 12
  %c4_1 = and i64 %cp12b4, 63
  %c4_1t = trunc i64 %c4_1 to i8
  %b4_1 = or i8 %c4_1t, -128
  %gp4_1 = getelementptr i8, ptr %1, i64 1
  store i8 %b4_1, ptr %gp4_1, align 1
  %cp6b4 = lshr i64 %0, 6
  %c4_2 = and i64 %cp6b4, 63
  %c4_2t = trunc i64 %c4_2 to i8
  %b4_2 = or i8 %c4_2t, -128
  %gp4_2 = getelementptr i8, ptr %1, i64 2
  store i8 %b4_2, ptr %gp4_2, align 1
  %c4_3 = and i64 %0, 63
  %c4_3t = trunc i64 %c4_3 to i8
  %b4_3 = or i8 %c4_3t, -128
  %gp4_3 = getelementptr i8, ptr %1, i64 3
  store i8 %b4_3, ptr %gp4_3, align 1
  ret i64 4
}

define i64 @action_utf8_byte_len(i8 %0) {
entry:
  %zext = zext i8 %0 to i64
  %and80 = and i64 %zext, 128
  %is_ascii = icmp eq i64 %and80, 0
  %andE0 = and i64 %zext, 224
  %is_2b = icmp eq i64 %andE0, 192
  %andF0 = and i64 %zext, 240
  %is_3b = icmp eq i64 %andF0, 224
  %andF8 = and i64 %zext, 248
  %is_4b = icmp eq i64 %andF8, 240
  %s3 = select i1 %is_3b, i64 3, i64 4
  %s2 = select i1 %is_2b, i64 2, i64 %s3
  %s1 = select i1 %is_ascii, i64 1, i64 %s2
  ret i64 %s1
}

declare i32 @sprintf(ptr, ptr, ...)

declare i64 @strlen(ptr)

declare ptr @memcpy(ptr, ptr, i64)

declare double @pow(double, double)

declare ptr @fopen(ptr, ptr)

declare i32 @fclose(ptr)

declare ptr @fgets(ptr, i32, ptr)

declare i64 @fread(ptr, i64, i64, ptr)

declare i64 @fwrite(ptr, i64, i64, ptr)

declare i32 @fseek(ptr, i64, i32)

declare i64 @ftell(ptr)

declare i32 @remove(ptr)

declare double @strtod(ptr, ptr)

declare i64 @strftime(ptr, i64, ptr, ptr)

declare ptr @strptime(ptr, ptr, ptr)

declare double @sqrt(double)

declare double @sin(double)

declare double @cos(double)

declare double @tan(double)

declare double @asin(double)

declare double @acos(double)

declare double @atan(double)

declare double @atan2(double, double)

declare double @log(double)

declare double @log2(double)

declare double @log10(double)

declare double @exp(double)

declare double @floor(double)

declare double @ceil(double)

declare double @round(double)

declare double @cbrt(double)

declare i32 @action_mutex_init(ptr, ptr)

declare i32 @action_mutex_lock(ptr)

declare i32 @action_mutex_unlock(ptr)

declare i32 @action_mutex_destroy(ptr)

declare i32 @action_cond_init(ptr, ptr)

declare i32 @action_cond_wait(ptr, ptr)

declare i32 @action_cond_signal(ptr)

declare i32 @action_cond_broadcast(ptr)

declare i32 @action_cond_destroy(ptr)

declare i32 @action_thread_create(ptr, ptr, ptr, ptr)

declare i32 @action_thread_join(i64, ptr)

declare i32 @action_thread_detach(i64)

declare i32 @action_thread_cancel(i64)

declare i32 @action_sleep_us(i32)

declare i32 @action_clock_gettime(i32, ptr)

declare ptr @memmove(ptr, ptr, i64)

declare ptr @action_http_request(ptr, ptr, ptr, ptr, i64)

declare void @action_http_free(ptr)

declare i64 @action_test_ping()

declare ptr @action_json_parse(ptr)

declare ptr @action_json_stringify(ptr)

declare void @action_json_free(ptr)

declare i64 @action_json_type(ptr)

declare ptr @action_json_get(ptr, ptr)

declare ptr @action_json_get_idx(ptr, i64)

declare ptr @action_json_as_str(ptr)

declare double @action_json_as_float(ptr)

declare i64 @action_json_as_bool(ptr)

declare i64 @action_json_len(ptr)

define void @action_print_int(i64 %0) {
entry:
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %0)
  ret void
}

define void @action_print_float(double %0) {
entry:
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_float, double %0)
  ret void
}

define void @action_print_bool(i1 %0) {
entry:
  br i1 %0, label %true_branch, label %false_branch

true_branch:                                      ; preds = %entry
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_str, ptr @.str_true)
  ret void

false_branch:                                     ; preds = %entry
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_str, ptr @.str_false)
  ret void
}

define void @action_print_string(%__action_str %0) {
entry:
  %data = extractvalue %__action_str %0, 1
  %is_null = icmp eq ptr %data, null
  br i1 %is_null, label %print_int, label %print_str

print_str:                                        ; preds = %entry
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_str, ptr %data)
  ret void

print_int:                                        ; preds = %entry
  %tag = extractvalue %__action_str %0, 0
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %tag)
  ret void
}

define void @action_println() {
entry:
  %0 = call i32 (ptr, ...) @printf(ptr @.fmt_nl)
  ret void
}

define void @action_list_print({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_lb)
  %lpi = alloca i64, align 8
  store i64 0, ptr %lpi, align 4
  br label %lphdr

lphdr:                                            ; preds = %lpval, %entry
  %lpiv = load i64, ptr %lpi, align 4
  %lpcond = icmp slt i64 %lpiv, %len
  br i1 %lpcond, label %lpbdy, label %lpext

lpbdy:                                            ; preds = %lphdr
  %is_first = icmp eq i64 %lpiv, 0
  br i1 %is_first, label %lpval, label %lpsep

lpext:                                            ; preds = %lphdr
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_rb)
  ret void

lpsep:                                            ; preds = %lpbdy
  %3 = call i32 (ptr, ...) @printf(ptr @.fmt_sep)
  br label %lpval

lpval:                                            ; preds = %lpsep, %lpbdy
  %lpep = getelementptr %__action_str, ptr %data, i64 %lpiv
  %lpe = load %__action_str, ptr %lpep, align 8
  %lptag = extractvalue %__action_str %lpe, 0
  %4 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %lptag)
  %lpnext = add i64 %lpiv, 1
  store i64 %lpnext, ptr %lpi, align 4
  br label %lphdr
}

define void @action_print_task({ i64, i64, i64, i64, { ptr, i64, i64 } } %0) {
entry:
  %done = extractvalue { i64, i64, i64, i64, { ptr, i64, i64 } } %0, 1
  %canc = extractvalue { i64, i64, i64, i64, { ptr, i64, i64 } } %0, 2
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_task_pre)
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %done)
  %3 = call i32 (ptr, ...) @printf(ptr @.fmt_task_mid)
  %4 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %canc)
  %5 = call i32 (ptr, ...) @printf(ptr @.fmt_task_suf)
  ret void
}

define void @action_print_struct() {
entry:
  %0 = call i32 (ptr, ...) @printf(ptr @.fmt_struct)
  ret void
}

define void @action_print_enum({ i64, ptr } %0) {
entry:
  %tag = extractvalue { i64, ptr } %0, 0
  %data = extractvalue { i64, ptr } %0, 1
  %is_null = icmp eq ptr %data, null
  br i1 %is_null, label %no_data, label %has_data

has_data:                                         ; preds = %entry
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_pre)
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %tag)
  %3 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_lp)
  %val = load i64, ptr %data, align 4
  %4 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %val)
  %5 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_rp)
  br label %merge

no_data:                                          ; preds = %entry
  %6 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_pre)
  %7 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %tag)
  %8 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_gt)
  br label %merge

merge:                                            ; preds = %no_data, %has_data
  ret void
}

define void @action_print_enum_float({ i64, ptr } %0) {
entry:
  %tag = extractvalue { i64, ptr } %0, 0
  %data = extractvalue { i64, ptr } %0, 1
  %is_null_f = icmp eq ptr %data, null
  br i1 %is_null_f, label %no_data, label %has_data

has_data:                                         ; preds = %entry
  %1 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_pre)
  %2 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %tag)
  %3 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_lp)
  %valf = load double, ptr %data, align 8
  %4 = call i32 (ptr, ...) @printf(ptr @.fmt_float, double %valf)
  %5 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_rp)
  br label %merge

no_data:                                          ; preds = %entry
  %6 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_pre)
  %7 = call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 %tag)
  %8 = call i32 (ptr, ...) @printf(ptr @.fmt_ev_gt)
  br label %merge

merge:                                            ; preds = %no_data, %has_data
  ret void
}

define %__action_str @action_string_create(ptr %0, i64 %1) {
entry:
  %alloc_size = add i64 %1, 1
  %buf = call ptr @action_malloc_rc(i64 %alloc_size)
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %buf, ptr align 1 %0, i64 %1, i1 false)
  %null_pos = getelementptr i8, ptr %buf, i64 %1
  store i8 0, ptr %null_pos, align 1
  %r1 = insertvalue %__action_str undef, i64 %1, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias writeonly captures(none), ptr noalias readonly captures(none), i64, i1 immarg) #0

define %__action_str @action_string_concat(%__action_str %0, %__action_str %1) {
entry:
  %len1 = extractvalue %__action_str %0, 0
  %data1 = extractvalue %__action_str %0, 1
  %len2 = extractvalue %__action_str %1, 0
  %data2 = extractvalue %__action_str %1, 1
  %total = add i64 %len1, %len2
  %alloc_size = add i64 %total, 1
  %buf = call ptr @action_malloc_rc(i64 %alloc_size)
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %buf, ptr align 1 %data1, i64 %len1, i1 false)
  %offset = getelementptr i8, ptr %buf, i64 %len1
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %offset, ptr align 1 %data2, i64 %len2, i1 false)
  %null_pos = getelementptr i8, ptr %buf, i64 %total
  store i8 0, ptr %null_pos, align 1
  %r1 = insertvalue %__action_str undef, i64 %total, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

define i1 @action_string_eq(%__action_str %0, %__action_str %1) {
entry:
  %len1 = extractvalue %__action_str %0, 0
  %len2 = extractvalue %__action_str %1, 0
  %len_eq = icmp eq i64 %len1, %len2
  br i1 %len_eq, label %compare, label %false

compare:                                          ; preds = %entry
  %is_empty = icmp eq i64 %len1, 0
  br i1 %is_empty, label %true, label %check_ptr

check_ptr:                                        ; preds = %compare
  %data1 = extractvalue %__action_str %0, 1
  %data2 = extractvalue %__action_str %1, 1
  %d1_null = icmp eq ptr %data1, null
  %d2_null = icmp eq ptr %data2, null
  %any_null = or i1 %d1_null, %d2_null
  br i1 %any_null, label %true, label %do_memcmp

do_memcmp:                                        ; preds = %check_ptr
  %cmp = call i32 @memcmp(ptr %data1, ptr %data2, i64 %len1)
  %content_eq = icmp eq i32 %cmp, 0
  br label %end

true:                                             ; preds = %check_ptr, %compare
  br label %end

false:                                            ; preds = %entry
  br label %end

end:                                              ; preds = %false, %true, %do_memcmp
  %eq_result = phi i1 [ true, %true ], [ false, %false ], [ %content_eq, %do_memcmp ]
  ret i1 %eq_result
}

define i64 @action_string_len(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  ret i64 %len
}

define %__action_str @action_int_to_string(i64 %0) {
entry:
  %buf = call ptr @action_malloc_rc(i64 32)
  %1 = call i32 (ptr, ptr, ...) @sprintf(ptr %buf, ptr @.fmt_int_str, i64 %0)
  %len = call i64 @strlen(ptr %buf)
  %r1 = insertvalue %__action_str undef, i64 %len, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

define %__action_str @action_float_to_string(double %0) {
entry:
  %buf = call ptr @action_malloc_rc(i64 32)
  %1 = call i32 (ptr, ptr, ...) @sprintf(ptr %buf, ptr @.fmt_float_str, double %0)
  %len = call i64 @strlen(ptr %buf)
  %r1 = insertvalue %__action_str undef, i64 %len, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

define i64 @action_int_pow(i64 %0, i64 %1) {
entry:
  %result = alloca i64, align 8
  %b = alloca i64, align 8
  %e = alloca i64, align 8
  store i64 1, ptr %result, align 4
  store i64 %0, ptr %b, align 4
  store i64 %1, ptr %e, align 4
  %neg = icmp slt i64 %1, 0
  br i1 %neg, label %done, label %loop

loop:                                             ; preds = %after_mul, %entry
  %e_cur = load i64, ptr %e, align 4
  %gt = icmp sgt i64 %e_cur, 0
  br i1 %gt, label %odd, label %done

odd:                                              ; preds = %loop
  %e_val = load i64, ptr %e, align 4
  %odd1 = and i64 %e_val, 1
  %odd_cmp = icmp eq i64 %odd1, 1
  br i1 %odd_cmp, label %mul, label %after_mul

after_mul:                                        ; preds = %mul, %odd
  %b_val = load i64, ptr %b, align 4
  %sq = mul i64 %b_val, %b_val
  store i64 %sq, ptr %b, align 4
  %e_val2 = load i64, ptr %e, align 4
  %half = sdiv i64 %e_val2, 2
  store i64 %half, ptr %e, align 4
  br label %loop

done:                                             ; preds = %loop, %entry
  %done_val = load i64, ptr %result, align 4
  ret i64 %done_val

mul:                                              ; preds = %odd
  %cur_r = load i64, ptr %result, align 4
  %cur_b = load i64, ptr %b, align 4
  %mul_r = mul i64 %cur_r, %cur_b
  store i64 %mul_r, ptr %result, align 4
  br label %after_mul
}

define { ptr, i64, i64 } @action_list_create(i64 %0) {
entry:
  %leaf = call ptr @action_malloc_rc(i64 ptrtoint (ptr getelementptr ({ i32, i32, [64 x %__action_str] }, ptr null, i32 1) to i64))
  store i64 0, ptr %leaf, align 4
  %r1 = insertvalue { ptr, i64, i64 } undef, ptr %leaf, 0
  %r2 = insertvalue { ptr, i64, i64 } %r1, i64 0, 1
  %r3 = insertvalue { ptr, i64, i64 } %r2, i64 0, 2
  ret { ptr, i64, i64 } %r3
}

define { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %0, %__action_str %1) {
entry:
  %node = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %height = extractvalue { ptr, i64, i64 } %0, 2
  %is_h0 = icmp eq i64 %height, 0
  br i1 %is_h0, label %h0, label %hgt0

h0:                                               ; preds = %entry
  %node_int = ptrtoint ptr %node to i64
  %rc_addr = sub i64 %node_int, 8
  %rc_ptr = inttoptr i64 %rc_addr to ptr
  %rc_val = load i64, ptr %rc_ptr, align 4
  %need_cow = icmp sgt i64 %rc_val, 1
  br i1 %need_cow, label %h0_cow, label %h0_room

h0_cow:                                           ; preds = %h0
  %new_leaf = call ptr @action_malloc_rc(i64 ptrtoint (ptr getelementptr ({ i32, i32, [64 x %__action_str] }, ptr null, i32 1) to i64))
  %2 = call ptr @memcpy(ptr %new_leaf, ptr %node, i64 ptrtoint (ptr getelementptr ({ i32, i32, [64 x %__action_str] }, ptr null, i32 1) to i64))
  %new_rc = sub i64 %rc_val, 1
  store i64 %new_rc, ptr %rc_ptr, align 4
  br label %h0_room

h0_room:                                          ; preds = %h0_cow, %h0
  %phi_leaf = phi ptr [ %node, %h0 ], [ %new_leaf, %h0_cow ]
  %count_val = load i64, ptr %phi_leaf, align 4
  %is_full = icmp sge i64 %count_val, 64
  br i1 %is_full, label %h0_full, label %h0_done

h0_full:                                          ; preds = %h0_room
  %nl2 = call ptr @action_malloc_rc(i64 ptrtoint (ptr getelementptr ({ i32, i32, [64 x %__action_str] }, ptr null, i32 1) to i64))
  %src_base = getelementptr i8, ptr %phi_leaf, i64 8
  %src32 = getelementptr %__action_str, ptr %src_base, i64 32
  %dst_base = getelementptr i8, ptr %nl2, i64 8
  %dst0 = getelementptr %__action_str, ptr %dst_base, i64 0
  %3 = call ptr @memcpy(ptr %dst0, ptr %src32, i64 512)
  %nl2_eb = getelementptr i8, ptr %nl2, i64 8
  %nl2e32 = getelementptr %__action_str, ptr %nl2_eb, i64 32
  store %__action_str %1, ptr %nl2e32, align 8
  store i64 32, ptr %phi_leaf, align 4
  store i64 33, ptr %nl2, align 4
  %intl = call ptr @action_malloc_rc(i64 ptrtoint (ptr getelementptr ({ i32, i32, i64, [64 x { ptr, i64 }] }, ptr null, i32 1) to i64))
  store i64 2, ptr %intl, align 4
  %total_p = getelementptr i64, ptr %intl, i64 8
  store i64 65, ptr %total_p, align 4
  %children_base = getelementptr i8, ptr %intl, i64 16
  %c0 = getelementptr { ptr, i64 }, ptr %children_base, i64 0
  store ptr %phi_leaf, ptr %c0, align 8
  %c0t = getelementptr i64, ptr %c0, i64 8
  store i64 32, ptr %c0t, align 4
  %c1 = getelementptr { ptr, i64 }, ptr %children_base, i64 1
  store ptr %nl2, ptr %c1, align 8
  %c1t = getelementptr i64, ptr %c1, i64 8
  store i64 33, ptr %c1t, align 4
  %new_total = add i64 %len, 1
  %sr1 = insertvalue { ptr, i64, i64 } undef, ptr %intl, 0
  %sr2 = insertvalue { ptr, i64, i64 } %sr1, i64 %new_total, 1
  %sr3 = insertvalue { ptr, i64, i64 } %sr2, i64 1, 2
  ret { ptr, i64, i64 } %sr3

h0_done:                                          ; preds = %h0_room
  %elem_base = getelementptr i8, ptr %phi_leaf, i64 8
  %elem_gep = getelementptr %__action_str, ptr %elem_base, i64 %count_val
  store %__action_str %1, ptr %elem_gep, align 8
  %new_count = add i64 %count_val, 1
  store i64 %new_count, ptr %phi_leaf, align 4
  %nt_h0 = add i64 %len, 1
  %h0r1 = insertvalue { ptr, i64, i64 } undef, ptr %phi_leaf, 0
  %h0r2 = insertvalue { ptr, i64, i64 } %h0r1, i64 %nt_h0, 1
  %h0r3 = insertvalue { ptr, i64, i64 } %h0r2, i64 0, 2
  ret { ptr, i64, i64 } %h0r3

hgt0:                                             ; preds = %entry
  br label %hgt0_done

hgt0_done:                                        ; preds = %hgt0
  %g1 = insertvalue { ptr, i64, i64 } undef, ptr %node, 0
  %g2 = insertvalue { ptr, i64, i64 } %g1, i64 %len, 1
  %g3 = insertvalue { ptr, i64, i64 } %g2, i64 %height, 2
  ret { ptr, i64, i64 } %g3
}

define %__action_str @action_list_get({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %node = extractvalue { ptr, i64, i64 } %0, 0
  %height = extractvalue { ptr, i64, i64 } %0, 2
  %is_h0 = icmp eq i64 %height, 0
  br i1 %is_h0, label %h0, label %hgt0

h0:                                               ; preds = %entry
  %elem_base = getelementptr i8, ptr %node, i64 8
  %elem_ptr = getelementptr %__action_str, ptr %elem_base, i64 %1
  %elem = load %__action_str, ptr %elem_ptr, align 8
  br label %ret

hgt0:                                             ; preds = %entry
  br label %hgt0_loop

hgt0_loop:                                        ; preds = %scan_found, %hgt0
  %phi_node = phi ptr [ %node, %hgt0 ], [ %child_p, %scan_found ]
  %phi_height = phi i64 [ %height, %hgt0 ], [ %new_h, %scan_found ]
  %phi_idx = phi i64 [ %1, %hgt0 ], [ %new_idx, %scan_found ]
  %is_leaf = icmp eq i64 %phi_height, 0
  br i1 %is_leaf, label %hgt0_found, label %hgt0_next

hgt0_found:                                       ; preds = %hgt0_loop
  %feb = getelementptr i8, ptr %phi_node, i64 8
  %fe_p = getelementptr %__action_str, ptr %feb, i64 %phi_idx
  %fe = load %__action_str, ptr %fe_p, align 8
  br label %ret

hgt0_next:                                        ; preds = %hgt0_loop
  %intl_count = load i64, ptr %phi_node, align 4
  br label %scan_loop

ret:                                              ; preds = %hgt0_found, %h0
  %phi_ret = phi %__action_str [ %elem, %h0 ], [ %fe, %hgt0_found ]
  ret %__action_str %phi_ret

scan_loop:                                        ; preds = %scan_next, %hgt0_next
  %phi_i = phi i64 [ 0, %hgt0_next ], [ %next_i, %scan_next ]
  %phi_acc = phi i64 [ 0, %hgt0_next ], [ %new_acc, %scan_next ]
  %done_scan = icmp sge i64 %phi_i, %intl_count
  br i1 %done_scan, label %scan_found, label %scan_body

scan_body:                                        ; preds = %scan_loop
  %scb = getelementptr i8, ptr %phi_node, i64 16
  %cep = getelementptr { ptr, i64 }, ptr %scb, i64 %phi_i
  %ce = load { ptr, i64 }, ptr %cep, align 8
  %ct = extractvalue { ptr, i64 } %ce, 1
  %new_acc = add i64 %phi_acc, %ct
  %found_child = icmp slt i64 %phi_idx, %new_acc
  br i1 %found_child, label %scan_found, label %scan_next

scan_found:                                       ; preds = %scan_body, %scan_loop
  %phi_fi = phi i64 [ %phi_i, %scan_body ], [ %phi_i, %scan_loop ]
  %phi_fa = phi i64 [ %phi_acc, %scan_body ], [ %phi_acc, %scan_loop ]
  %fceb = getelementptr i8, ptr %phi_node, i64 16
  %fcep = getelementptr { ptr, i64 }, ptr %fceb, i64 %phi_fi
  %fce = load { ptr, i64 }, ptr %fcep, align 8
  %child_p = extractvalue { ptr, i64 } %fce, 0
  %new_idx = sub i64 %phi_idx, %phi_fa
  %new_h = sub i64 %phi_height, 1
  br label %hgt0_loop

scan_next:                                        ; preds = %scan_body
  %next_i = add i64 %phi_i, 1
  br label %scan_loop
}

define %__action_str @action_list_head({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %none, label %has

has:                                              ; preds = %entry
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %height = extractvalue { ptr, i64, i64 } %0, 2
  br label %lh_loop

none:                                             ; preds = %entry
  ret %__action_str zeroinitializer

lh_loop:                                          ; preds = %lh_descend, %has
  %phi_node = phi ptr [ %data, %has ], [ %ch_ptr, %lh_descend ]
  %phi_rem = phi i64 [ %height, %has ], [ %new_rem, %lh_descend ]
  %at_leaf = icmp eq i64 %phi_rem, 0
  br i1 %at_leaf, label %lh_leaf, label %lh_descend

lh_leaf:                                          ; preds = %lh_loop
  %eb = getelementptr i8, ptr %phi_node, i64 8
  %elem_ptr = getelementptr %__action_str, ptr %eb, i64 0
  %val = load %__action_str, ptr %elem_ptr, align 8
  ret %__action_str %val

lh_descend:                                       ; preds = %lh_loop
  %ch_base = getelementptr i8, ptr %phi_node, i64 16
  %ch_ptr = load ptr, ptr %ch_base, align 8
  %new_rem = sub i64 %phi_rem, 1
  br label %lh_loop
}

define i64 @action_list_len({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  ret i64 %len
}

define i1 @action_list_contains({ ptr, i64, i64 } %0, %__action_str %1) {
entry:
  %lc_data = extractvalue { ptr, i64, i64 } %0, 0
  %lc_len = extractvalue { ptr, i64, i64 } %0, 1
  %lc_ktag = extractvalue %__action_str %1, 0
  %lc_kdata = extractvalue %__action_str %1, 1
  br label %lc_loop

lc_loop:                                          ; preds = %lc_next, %entry
  %lc_i = phi i64 [ 0, %entry ], [ %lc_ni, %lc_next ]
  %lc_ep = getelementptr %__action_str, ptr %lc_data, i64 %lc_i
  %lc_elem = load %__action_str, ptr %lc_ep, align 8
  %lc_etag = extractvalue %__action_str %lc_elem, 0
  %lc_edata = extractvalue %__action_str %lc_elem, 1
  %lc_teq = icmp eq i64 %lc_etag, %lc_ktag
  br i1 %lc_teq, label %lc_check, label %lc_next

lc_done:                                          ; preds = %lc_next
  ret i1 false

lc_next:                                          ; preds = %lc_str_check, %lc_content, %lc_loop
  %lc_ni = add i64 %lc_i, 1
  %lc_done1 = icmp sge i64 %lc_ni, %lc_len
  br i1 %lc_done1, label %lc_done, label %lc_loop

lc_check:                                         ; preds = %lc_loop
  %ed_null = icmp eq ptr %lc_edata, null
  %kd_null = icmp eq ptr %lc_kdata, null
  %both_null = and i1 %ed_null, %kd_null
  br i1 %both_null, label %lc_found, label %lc_content

lc_found:                                         ; preds = %lc_check
  ret i1 true

lc_content:                                       ; preds = %lc_check
  %ed_nn = xor i1 %ed_null, true
  %kd_nn = xor i1 %kd_null, true
  %both_nn = and i1 %ed_nn, %kd_nn
  br i1 %both_nn, label %lc_str_check, label %lc_next

lc_str_check:                                     ; preds = %lc_content
  %2 = call i1 @action_string_eq(%__action_str %lc_elem, %__action_str %1)
  br i1 %2, label %lc_str_found, label %lc_next

lc_str_found:                                     ; preds = %lc_str_check
  ret i1 true
}

define { ptr, i64, i64 } @action_list_reverse({ ptr, i64, i64 } %0) {
entry:
  %lr_data = extractvalue { ptr, i64, i64 } %0, 0
  %lr_len = extractvalue { ptr, i64, i64 } %0, 1
  %lr_cap = extractvalue { ptr, i64, i64 } %0, 2
  %lr_new = call { ptr, i64, i64 } @action_list_create(i64 %lr_cap)
  br label %lr_loop

lr_loop:                                          ; preds = %lr_loop, %entry
  %lr_i = phi i64 [ 0, %entry ], [ %lr_ni, %lr_loop ]
  %lr_list2 = phi { ptr, i64, i64 } [ %lr_new, %entry ], [ %lr_push, %lr_loop ]
  %lr_plus1 = add i64 %lr_i, 1
  %lr_rev_idx = sub i64 %lr_len, %lr_plus1
  %lr_ep = getelementptr %__action_str, ptr %lr_data, i64 %lr_rev_idx
  %lr_elem = load %__action_str, ptr %lr_ep, align 8
  %lr_push = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %lr_list2, %__action_str %lr_elem)
  %lr_ni = add i64 %lr_i, 1
  %lr_done1 = icmp sge i64 %lr_ni, %lr_len
  br i1 %lr_done1, label %lr_done, label %lr_loop

lr_done:                                          ; preds = %lr_loop
  %lr_final = phi { ptr, i64, i64 } [ %lr_push, %lr_loop ]
  ret { ptr, i64, i64 } %lr_final
}

define { ptr, i64, i64 } @action_list_range(i64 %0, i64 %1) {
entry:
  %rg_len = sub i64 %1, %0
  %rg_cap = add i64 %rg_len, 1
  %rg_list = call { ptr, i64, i64 } @action_list_create(i64 %rg_cap)
  %rg_check = icmp slt i64 %0, %1
  br i1 %rg_check, label %rg_loop, label %rg_done

rg_loop:                                          ; preds = %rg_loop, %entry
  %rg_i = phi i64 [ %0, %entry ], [ %rg_next, %rg_loop ]
  %rg_list2 = phi { ptr, i64, i64 } [ %rg_list, %entry ], [ %rg_push, %rg_loop ]
  %rg_fat_val = insertvalue %__action_str undef, i64 %rg_i, 0
  %rg_fat = insertvalue %__action_str %rg_fat_val, ptr null, 1
  %rg_push = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %rg_list2, %__action_str %rg_fat)
  %rg_next = add i64 %rg_i, 1
  %rg_done_cond = icmp sge i64 %rg_next, %1
  br i1 %rg_done_cond, label %rg_done, label %rg_loop

rg_done:                                          ; preds = %rg_loop, %entry
  %rg_final = phi { ptr, i64, i64 } [ %rg_list, %entry ], [ %rg_push, %rg_loop ]
  ret { ptr, i64, i64 } %rg_final
}

define { ptr, i64, i64 } @action_list_take({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %lt_len = extractvalue { ptr, i64, i64 } %0, 1
  %lt_cmp = icmp slt i64 %1, %lt_len
  %lt_actual = select i1 %lt_cmp, i64 %1, i64 %lt_len
  %lt_new = call { ptr, i64, i64 } @action_list_create(i64 %lt_actual)
  %lt_data = extractvalue { ptr, i64, i64 } %0, 0
  br label %lt_loop

lt_loop:                                          ; preds = %lt_loop, %entry
  %lt_i = phi i64 [ 0, %entry ], [ %lt_ni, %lt_loop ]
  %lt_cur = phi { ptr, i64, i64 } [ %lt_new, %entry ], [ %lt_push, %lt_loop ]
  %lt_ep = getelementptr %__action_str, ptr %lt_data, i64 %lt_i
  %lt_elem = load %__action_str, ptr %lt_ep, align 8
  %lt_push = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %lt_cur, %__action_str %lt_elem)
  %lt_ni = add i64 %lt_i, 1
  %lt_done1 = icmp sge i64 %lt_ni, %lt_actual
  br i1 %lt_done1, label %lt_done, label %lt_loop

lt_done:                                          ; preds = %lt_loop
  %lt_final = phi { ptr, i64, i64 } [ %lt_push, %lt_loop ]
  ret { ptr, i64, i64 } %lt_final
}

define { ptr, i64, i64 } @action_list_drop({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %ld_len = extractvalue { ptr, i64, i64 } %0, 1
  %ld_data = extractvalue { ptr, i64, i64 } %0, 0
  %ld_cmp = icmp slt i64 %1, %ld_len
  %ld_start = select i1 %ld_cmp, i64 %1, i64 %ld_len
  %ld_rem = sub i64 %ld_len, %ld_start
  %ld_cap = add i64 %ld_rem, 1
  %ld_new = call { ptr, i64, i64 } @action_list_create(i64 %ld_cap)
  br label %ld_loop

ld_loop:                                          ; preds = %ld_loop, %entry
  %ld_i = phi i64 [ 0, %entry ], [ %ld_ni, %ld_loop ]
  %ld_cur = phi { ptr, i64, i64 } [ %ld_new, %entry ], [ %ld_push, %ld_loop ]
  %ld_idx = add i64 %ld_i, %ld_start
  %ld_ep = getelementptr %__action_str, ptr %ld_data, i64 %ld_idx
  %ld_elem = load %__action_str, ptr %ld_ep, align 8
  %ld_push = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %ld_cur, %__action_str %ld_elem)
  %ld_ni = add i64 %ld_i, 1
  %ld_done1 = icmp sge i64 %ld_ni, %ld_rem
  br i1 %ld_done1, label %ld_done, label %ld_loop

ld_done:                                          ; preds = %ld_loop
  %ld_final = phi { ptr, i64, i64 } [ %ld_push, %ld_loop ]
  ret { ptr, i64, i64 } %ld_final
}

define i64 @abs(i64 %0) {
entry:
  %neg = sub i64 0, %0
  %is_neg = icmp slt i64 %0, 0
  %abs_result = select i1 %is_neg, i64 %neg, i64 %0
  ret i64 %abs_result
}

define i64 @min(i64 %0, i64 %1) {
entry:
  %lt = icmp slt i64 %0, %1
  %min_result = select i1 %lt, i64 %0, i64 %1
  ret i64 %min_result
}

define i64 @max(i64 %0, i64 %1) {
entry:
  %gt = icmp sgt i64 %0, %1
  %max_result = select i1 %gt, i64 %0, i64 %1
  ret i64 %max_result
}

define %__action_str @action_string_to_upper(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %data = extractvalue %__action_str %0, 1
  %alloc_len = add i64 %len, 1
  %new_buf = call ptr @action_malloc_rc(i64 %alloc_len)
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i_val = load i64, ptr %i, align 4
  %not_done = icmp ult i64 %i_val, %len
  br i1 %not_done, label %body, label %done

body:                                             ; preds = %loop
  %src_ptr = getelementptr i8, ptr %data, i64 %i_val
  %c = load i8, ptr %src_ptr, align 1
  %ge_a = icmp uge i8 %c, 97
  %le_z = icmp ule i8 %c, 122
  %is_lower = and i1 %ge_a, %le_z
  %upper_c = sub i8 %c, 32
  %conv = select i1 %is_lower, i8 %upper_c, i8 %c
  %dst_ptr = getelementptr i8, ptr %new_buf, i64 %i_val
  store i8 %conv, ptr %dst_ptr, align 1
  %next_i = add i64 %i_val, 1
  store i64 %next_i, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %null_ptr = getelementptr i8, ptr %new_buf, i64 %len
  store i8 0, ptr %null_ptr, align 1
  %r1 = insertvalue %__action_str undef, i64 %len, 0
  %r2 = insertvalue %__action_str %r1, ptr %new_buf, 1
  ret %__action_str %r2
}

define %__action_str @action_string_to_lower(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %data = extractvalue %__action_str %0, 1
  %alloc_len = add i64 %len, 1
  %new_buf = call ptr @action_malloc_rc(i64 %alloc_len)
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i_val = load i64, ptr %i, align 4
  %not_done = icmp ult i64 %i_val, %len
  br i1 %not_done, label %body, label %done

body:                                             ; preds = %loop
  %src_ptr = getelementptr i8, ptr %data, i64 %i_val
  %c = load i8, ptr %src_ptr, align 1
  %ge_A = icmp uge i8 %c, 65
  %le_Z = icmp ule i8 %c, 90
  %is_upper = and i1 %ge_A, %le_Z
  %lower_c = add i8 %c, 32
  %conv = select i1 %is_upper, i8 %lower_c, i8 %c
  %dst_ptr = getelementptr i8, ptr %new_buf, i64 %i_val
  store i8 %conv, ptr %dst_ptr, align 1
  %next_i = add i64 %i_val, 1
  store i64 %next_i, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %null_ptr = getelementptr i8, ptr %new_buf, i64 %len
  store i8 0, ptr %null_ptr, align 1
  %r1 = insertvalue %__action_str undef, i64 %len, 0
  %r2 = insertvalue %__action_str %r1, ptr %new_buf, 1
  ret %__action_str %r2
}

define %__action_str @action_string_trim(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %data = extractvalue %__action_str %0, 1
  %start_idx = alloca i64, align 8
  store i64 0, ptr %start_idx, align 4
  br label %find_start_hdr

find_start_hdr:                                   ; preds = %find_start_body, %entry
  %si = load i64, ptr %start_idx, align 4
  %si_lt_len = icmp ult i64 %si, %len
  br i1 %si_lt_len, label %find_start_body, label %start_done

find_start_body:                                  ; preds = %find_start_hdr
  %sp = getelementptr i8, ptr %data, i64 %si
  %sc = load i8, ptr %sp, align 1
  %is_sp = icmp eq i8 %sc, 32
  %is_tab = icmp eq i8 %sc, 9
  %is_nl = icmp eq i8 %sc, 10
  %is_cr = icmp eq i8 %sc, 13
  %ws1 = or i1 %is_sp, %is_tab
  %ws2 = or i1 %is_nl, %is_cr
  %is_ws = or i1 %ws1, %ws2
  %si_plus1 = add i64 %si, 1
  %new_si = select i1 %is_ws, i64 %si_plus1, i64 %si
  store i64 %new_si, ptr %start_idx, align 4
  br i1 %is_ws, label %find_start_hdr, label %start_done

start_done:                                       ; preds = %find_start_body, %find_start_hdr
  %end_idx = alloca i64, align 8
  store i64 %len, ptr %end_idx, align 4
  %final_si = load i64, ptr %start_idx, align 4
  br label %find_end_hdr

find_end_hdr:                                     ; preds = %find_end_body, %start_done
  %ei = load i64, ptr %end_idx, align 4
  %ei_gt_si = icmp ugt i64 %ei, %final_si
  br i1 %ei_gt_si, label %find_end_body, label %end_done

find_end_body:                                    ; preds = %find_end_hdr
  %ei_minus1 = sub i64 %ei, 1
  %ep = getelementptr i8, ptr %data, i64 %ei_minus1
  %ec = load i8, ptr %ep, align 1
  %is_sp1 = icmp eq i8 %ec, 32
  %is_tab2 = icmp eq i8 %ec, 9
  %is_nl3 = icmp eq i8 %ec, 10
  %is_cr4 = icmp eq i8 %ec, 13
  %ws15 = or i1 %is_sp1, %is_tab2
  %ws26 = or i1 %is_nl3, %is_cr4
  %is_ws7 = or i1 %ws15, %ws26
  %new_ei = select i1 %is_ws7, i64 %ei_minus1, i64 %ei
  store i64 %new_ei, ptr %end_idx, align 4
  br i1 %is_ws7, label %find_end_hdr, label %end_done

end_done:                                         ; preds = %find_end_body, %find_end_hdr
  %final_ei = load i64, ptr %end_idx, align 4
  %new_len = sub i64 %final_ei, %final_si
  %alloc_len = add i64 %new_len, 1
  %new_buf = call ptr @action_malloc_rc(i64 %alloc_len)
  %src_offset = getelementptr i8, ptr %data, i64 %final_si
  %1 = call ptr @memcpy(ptr %new_buf, ptr %src_offset, i64 %new_len)
  %null_ptr = getelementptr i8, ptr %new_buf, i64 %new_len
  store i8 0, ptr %null_ptr, align 1
  %r1 = insertvalue %__action_str undef, i64 %new_len, 0
  %r2 = insertvalue %__action_str %r1, ptr %new_buf, 1
  ret %__action_str %r2
}

define { ptr, i64, i64 } @action_map_create(i64 %0) {
entry:
  %m_data_size = mul i64 %0, 32
  %m_data = call ptr @action_malloc_rc(i64 %m_data_size)
  %r1 = insertvalue { ptr, i64, i64 } undef, ptr %m_data, 0
  %r2 = insertvalue { ptr, i64, i64 } %r1, i64 0, 1
  %r3 = insertvalue { ptr, i64, i64 } %r2, i64 %0, 2
  ret { ptr, i64, i64 } %r3
}

define { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %0, %__action_str %1, %__action_str %2) {
entry:
  %d = extractvalue { ptr, i64, i64 } %0, 0
  %l = extractvalue { ptr, i64, i64 } %0, 1
  %c = extractvalue { ptr, i64, i64 } %0, 2
  %mi_data_int = ptrtoint ptr %d to i64
  %mi_rc_addr = sub i64 %mi_data_int, 8
  %mi_rc_ptr = inttoptr i64 %mi_rc_addr to ptr
  %mi_rc_val = load i64, ptr %mi_rc_ptr, align 4
  %mi_need_cow = icmp sgt i64 %mi_rc_val, 1
  br i1 %mi_need_cow, label %cow_clone, label %merge

cow_clone:                                        ; preds = %entry
  %mi_cow_size = mul i64 %c, 32
  %mi_new_data = call ptr @action_malloc_rc(i64 %mi_cow_size)
  %3 = call ptr @memcpy(ptr %mi_new_data, ptr %d, i64 %mi_cow_size)
  %mi_cow_old_rc = load i64, ptr %mi_rc_ptr, align 4
  %mi_cow_new_rc = sub i64 %mi_cow_old_rc, 1
  store i64 %mi_cow_new_rc, ptr %mi_rc_ptr, align 4
  br label %merge

merge:                                            ; preds = %cow_clone, %entry
  %mi_data_phi = phi ptr [ %d, %entry ], [ %mi_new_data, %cow_clone ]
  %kt = extractvalue %__action_str %1, 0
  %kp = extractvalue %__action_str %1, 1
  %vt = extractvalue %__action_str %2, 0
  %vp = extractvalue %__action_str %2, 1
  %kp_i64 = ptrtoint ptr %kp to i64
  %vp_i64 = ptrtoint ptr %vp to i64
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %search

search:                                           ; preds = %next, %merge
  %iv = load i64, ptr %i, align 4
  %cond = icmp slt i64 %iv, %l
  br i1 %cond, label %body, label %append_ck

body:                                             ; preds = %search
  %off = mul i64 %iv, 4
  %etp = getelementptr i64, ptr %mi_data_phi, i64 %off
  %et = load i64, ptr %etp, align 4
  %teq = icmp eq i64 %et, %kt
  br i1 %teq, label %ckey, label %next

ckey:                                             ; preds = %body
  %off1 = add i64 %off, 1
  %epp = getelementptr i64, ptr %mi_data_phi, i64 %off1
  %ep = load i64, ptr %epp, align 4
  %kpz = icmp eq i64 %kp_i64, 0
  %ek1 = insertvalue %__action_str undef, i64 %et, 0
  %ep_ptr = inttoptr i64 %ep to ptr
  %ek2 = insertvalue %__action_str %ek1, ptr %ep_ptr, 1
  %seq = call i1 @action_string_eq(%__action_str %ek2, %__action_str %1)
  %feq = select i1 %kpz, i1 %teq, i1 %seq
  br i1 %feq, label %update, label %next

update:                                           ; preds = %ckey
  %off2 = add i64 %off, 2
  %vtp = getelementptr i64, ptr %mi_data_phi, i64 %off2
  store i64 %vt, ptr %vtp, align 4
  %off3 = add i64 %off, 3
  %vpp = getelementptr i64, ptr %mi_data_phi, i64 %off3
  store i64 %vp_i64, ptr %vpp, align 4
  %r1 = insertvalue { ptr, i64, i64 } undef, ptr %mi_data_phi, 0
  %r2 = insertvalue { ptr, i64, i64 } %r1, i64 %l, 1
  %r3 = insertvalue { ptr, i64, i64 } %r2, i64 %c, 2
  ret { ptr, i64, i64 } %r3

next:                                             ; preds = %ckey, %body
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %i, align 4
  br label %search

append_ck:                                        ; preds = %search
  %need_grow = icmp sge i64 %l, %c
  br i1 %need_grow, label %append_grow, label %append_store

append_grow:                                      ; preds = %append_ck
  %cap_small = icmp slt i64 %c, 4
  %cap2x = mul i64 %c, 2
  %new_cap = select i1 %cap_small, i64 4, i64 %cap2x
  %data_size = mul i64 %new_cap, 32
  %total_size = add i64 %data_size, 8
  %mi_data_int1 = ptrtoint ptr %mi_data_phi to i64
  %mi_orig_int = sub i64 %mi_data_int1, 8
  %mi_orig_ptr = inttoptr i64 %mi_orig_int to ptr
  %mi_new_orig = call ptr @realloc(ptr %mi_orig_ptr, i64 %total_size)
  %mi_new_orig_int = ptrtoint ptr %mi_new_orig to i64
  %mi_new_data_int = add i64 %mi_new_orig_int, 8
  %mi_new_data2 = inttoptr i64 %mi_new_data_int to ptr
  br label %append_store

append_store:                                     ; preds = %append_grow, %append_ck
  %phi_data = phi ptr [ %mi_data_phi, %append_ck ], [ %mi_new_data2, %append_grow ]
  %phi_di64 = phi ptr [ %mi_data_phi, %append_ck ], [ %mi_new_data2, %append_grow ]
  %phi_cap = phi i64 [ %c, %append_ck ], [ %new_cap, %append_grow ]
  %lo = mul i64 %l, 4
  %nkt = getelementptr i64, ptr %phi_di64, i64 %lo
  store i64 %kt, ptr %nkt, align 4
  %lo1 = add i64 %lo, 1
  %nkp = getelementptr i64, ptr %phi_di64, i64 %lo1
  store i64 %kp_i64, ptr %nkp, align 4
  %lo2 = add i64 %lo, 2
  %nvt = getelementptr i64, ptr %phi_di64, i64 %lo2
  store i64 %vt, ptr %nvt, align 4
  %lo3 = add i64 %lo, 3
  %nvp = getelementptr i64, ptr %phi_di64, i64 %lo3
  store i64 %vp_i64, ptr %nvp, align 4
  %nl = add i64 %l, 1
  %rr1 = insertvalue { ptr, i64, i64 } undef, ptr %phi_data, 0
  %rr2 = insertvalue { ptr, i64, i64 } %rr1, i64 %nl, 1
  %rr3 = insertvalue { ptr, i64, i64 } %rr2, i64 %phi_cap, 2
  ret { ptr, i64, i64 } %rr3
}

define %__action_str @action_map_get({ ptr, i64, i64 } %0, %__action_str %1) {
b0:
  %d = extractvalue { ptr, i64, i64 } %0, 0
  %l = extractvalue { ptr, i64, i64 } %0, 1
  %kt = extractvalue %__action_str %1, 0
  %kp = extractvalue %__action_str %1, 1
  %kp_i64 = ptrtoint ptr %kp to i64
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %b1

b1:                                               ; preds = %b5, %b0
  %iv = load i64, ptr %i, align 4
  %cond = icmp slt i64 %iv, %l
  br i1 %cond, label %b2, label %b6

b2:                                               ; preds = %b1
  %off = mul i64 %iv, 4
  %etp = getelementptr i64, ptr %d, i64 %off
  %et = load i64, ptr %etp, align 4
  %teq = icmp eq i64 %et, %kt
  br i1 %teq, label %b3, label %b5

b3:                                               ; preds = %b2
  %off1 = add i64 %off, 1
  %epp = getelementptr i64, ptr %d, i64 %off1
  %ep = load i64, ptr %epp, align 4
  %kpz = icmp eq i64 %kp_i64, 0
  %ek1 = insertvalue %__action_str undef, i64 %et, 0
  %ep_ptr = inttoptr i64 %ep to ptr
  %ek2 = insertvalue %__action_str %ek1, ptr %ep_ptr, 1
  %seq = call i1 @action_string_eq(%__action_str %ek2, %__action_str %1)
  %feq = select i1 %kpz, i1 %teq, i1 %seq
  br i1 %feq, label %b4, label %b5

b4:                                               ; preds = %b3
  %off2 = add i64 %off, 2
  %vtp = getelementptr i64, ptr %d, i64 %off2
  %vt = load i64, ptr %vtp, align 4
  %off3 = add i64 %off, 3
  %vpp = getelementptr i64, ptr %d, i64 %off3
  %vp = load i64, ptr %vpp, align 4
  %r1 = insertvalue %__action_str undef, i64 %vt, 0
  %vp_ptr = inttoptr i64 %vp to ptr
  %r2 = insertvalue %__action_str %r1, ptr %vp_ptr, 1
  ret %__action_str %r2

b5:                                               ; preds = %b3, %b2
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %i, align 4
  br label %b1

b6:                                               ; preds = %b1
  ret %__action_str zeroinitializer
}

define i1 @action_map_contains({ ptr, i64, i64 } %0, %__action_str %1) {
b0:
  %d = extractvalue { ptr, i64, i64 } %0, 0
  %l = extractvalue { ptr, i64, i64 } %0, 1
  %kt = extractvalue %__action_str %1, 0
  %kp = extractvalue %__action_str %1, 1
  %kp_i64 = ptrtoint ptr %kp to i64
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %b1

b1:                                               ; preds = %b5, %b0
  %iv = load i64, ptr %i, align 4
  %cond = icmp slt i64 %iv, %l
  br i1 %cond, label %b2, label %b6

b2:                                               ; preds = %b1
  %off = mul i64 %iv, 4
  %etp = getelementptr i64, ptr %d, i64 %off
  %et = load i64, ptr %etp, align 4
  %teq = icmp eq i64 %et, %kt
  br i1 %teq, label %b3, label %b5

b3:                                               ; preds = %b2
  %off1 = add i64 %off, 1
  %epp = getelementptr i64, ptr %d, i64 %off1
  %ep = load i64, ptr %epp, align 4
  %kpz = icmp eq i64 %kp_i64, 0
  %ek1 = insertvalue %__action_str undef, i64 %et, 0
  %ep_ptr = inttoptr i64 %ep to ptr
  %ek2 = insertvalue %__action_str %ek1, ptr %ep_ptr, 1
  %seq = call i1 @action_string_eq(%__action_str %ek2, %__action_str %1)
  %feq = select i1 %kpz, i1 %teq, i1 %seq
  br i1 %feq, label %b4, label %b5

b4:                                               ; preds = %b3
  ret i1 true

b5:                                               ; preds = %b3, %b2
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %i, align 4
  br label %b1

b6:                                               ; preds = %b1
  ret i1 false
}

define { ptr, i64, i64 } @action_map_remove({ ptr, i64, i64 } %0, %__action_str %1) {
b0:
  %d = extractvalue { ptr, i64, i64 } %0, 0
  %l = extractvalue { ptr, i64, i64 } %0, 1
  %c = extractvalue { ptr, i64, i64 } %0, 2
  %kt = extractvalue %__action_str %1, 0
  %kp = extractvalue %__action_str %1, 1
  %kp_i64 = ptrtoint ptr %kp to i64
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %b1

b1:                                               ; preds = %b6, %b0
  %iv = load i64, ptr %i, align 4
  %cond = icmp slt i64 %iv, %l
  br i1 %cond, label %b2, label %b7

b2:                                               ; preds = %b1
  %off = mul i64 %iv, 4
  %etp = getelementptr i64, ptr %d, i64 %off
  %et = load i64, ptr %etp, align 4
  %teq = icmp eq i64 %et, %kt
  br i1 %teq, label %b3, label %b6

b3:                                               ; preds = %b2
  %off1 = add i64 %off, 1
  %epp = getelementptr i64, ptr %d, i64 %off1
  %ep = load i64, ptr %epp, align 4
  %kpz = icmp eq i64 %kp_i64, 0
  %ek1 = insertvalue %__action_str undef, i64 %et, 0
  %ep_ptr = inttoptr i64 %ep to ptr
  %ek2 = insertvalue %__action_str %ek1, ptr %ep_ptr, 1
  %seq = call i1 @action_string_eq(%__action_str %ek2, %__action_str %1)
  %feq = select i1 %kpz, i1 %teq, i1 %seq
  br i1 %feq, label %b4, label %b6

b4:                                               ; preds = %b3
  %len_dec = sub i64 %l, 1
  %iv_p1 = add i64 %iv, 1
  %remaining = sub i64 %l, %iv_p1
  %has_rem = icmp sgt i64 %remaining, 0
  br i1 %has_rem, label %b5, label %b7

b5:                                               ; preds = %b4
  %src_off = mul i64 %iv_p1, 32
  %dst_off = mul i64 %iv, 32
  %src = getelementptr i8, ptr %d, i64 %src_off
  %dst = getelementptr i8, ptr %d, i64 %dst_off
  %rem_bytes = mul i64 %remaining, 32
  %2 = call ptr @memcpy(ptr %dst, ptr %src, i64 %rem_bytes)
  br label %b7

b6:                                               ; preds = %b3, %b2
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %i, align 4
  br label %b1

b7:                                               ; preds = %b5, %b4, %b1
  %ret_len = phi i64 [ %l, %b1 ], [ %len_dec, %b4 ], [ %len_dec, %b5 ]
  %r1 = insertvalue { ptr, i64, i64 } undef, ptr %d, 0
  %r2 = insertvalue { ptr, i64, i64 } %r1, i64 %ret_len, 1
  %r3 = insertvalue { ptr, i64, i64 } %r2, i64 %c, 2
  ret { ptr, i64, i64 } %r3
}

define i1 @action_string_starts_with(%__action_str %0, %__action_str %1) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %plen = extractvalue %__action_str %1, 0
  %sdata = extractvalue %__action_str %0, 1
  %pdata = extractvalue %__action_str %1, 1
  %len_ok = icmp uge i64 %slen, %plen
  br i1 %len_ok, label %check, label %false

check:                                            ; preds = %entry
  %pz = icmp eq i64 %plen, 0
  br i1 %pz, label %done, label %cmp

cmp:                                              ; preds = %check
  %mc = call i32 @memcmp(ptr %sdata, ptr %pdata, i64 %plen)
  %eq = icmp eq i32 %mc, 0
  br label %done

false:                                            ; preds = %entry
  br label %done

done:                                             ; preds = %false, %cmp, %check
  %sw_result = phi i1 [ %pz, %check ], [ %eq, %cmp ], [ false, %false ]
  ret i1 %sw_result
}

define i1 @action_string_ends_with(%__action_str %0, %__action_str %1) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %suflen = extractvalue %__action_str %1, 0
  %sdata = extractvalue %__action_str %0, 1
  %sufdata = extractvalue %__action_str %1, 1
  %len_ok = icmp uge i64 %slen, %suflen
  br i1 %len_ok, label %check, label %false

check:                                            ; preds = %entry
  %sufz = icmp eq i64 %suflen, 0
  br i1 %sufz, label %done, label %cmp

cmp:                                              ; preds = %check
  %off = sub i64 %slen, %suflen
  %sp = getelementptr i8, ptr %sdata, i64 %off
  %mc = call i32 @memcmp(ptr %sp, ptr %sufdata, i64 %suflen)
  %eq = icmp eq i32 %mc, 0
  br label %done

false:                                            ; preds = %entry
  br label %done

done:                                             ; preds = %false, %cmp, %check
  %ew_result = phi i1 [ %sufz, %check ], [ %eq, %cmp ], [ false, %false ]
  ret i1 %ew_result
}

define %__action_str @action_string_substring(%__action_str %0, i64 %1, i64 %2) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sdata = extractvalue %__action_str %0, 1
  %start_ok = icmp ult i64 %1, %slen
  %end = add i64 %1, %2
  %end_ok = icmp ule i64 %end, %slen
  %clamped_end = select i1 %end_ok, i64 %end, i64 %slen
  %actual_len = sub i64 %clamped_end, %1
  %clamped_start = select i1 %start_ok, i64 %1, i64 %slen
  %zero_len = icmp eq i64 %actual_len, 0
  %alc = add i64 %actual_len, 1
  %buf = call ptr @action_malloc_rc(i64 %alc)
  %src = getelementptr i8, ptr %sdata, i64 %clamped_start
  %3 = call ptr @memcpy(ptr %buf, ptr %src, i64 %actual_len)
  %null = getelementptr i8, ptr %buf, i64 %actual_len
  store i8 0, ptr %null, align 1
  %r1 = insertvalue %__action_str undef, i64 %actual_len, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

define { i64, i1 } @action_parse_int(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %data = extractvalue %__action_str %0, 1
  %result = alloca i64, align 8
  %sign = alloca i64, align 8
  %i = alloca i64, align 8
  %valid = alloca i1, align 1
  store i64 0, ptr %result, align 4
  store i64 1, ptr %sign, align 4
  store i64 0, ptr %i, align 4
  store i1 false, ptr %valid, align 1
  %has_chars = icmp ugt i64 %len, 0
  br i1 %has_chars, label %check_sign, label %done

check_sign:                                       ; preds = %entry
  %first = load i8, ptr %data, align 1
  %is_minus = icmp eq i8 %first, 45
  br label %setup

setup:                                            ; preds = %check_sign
  %sign_val = select i1 %is_minus, i64 -1, i64 1
  %start_i = select i1 %is_minus, i64 1, i64 0
  store i64 %sign_val, ptr %sign, align 4
  store i64 %start_i, ptr %i, align 4
  br label %loop_hdr

loop_hdr:                                         ; preds = %body_next, %setup
  %iv = load i64, ptr %i, align 4
  %not_done = icmp ult i64 %iv, %len
  br i1 %not_done, label %loop_body, label %done

loop_body:                                        ; preds = %loop_hdr
  %chp = getelementptr i8, ptr %data, i64 %iv
  %ch = load i8, ptr %chp, align 1
  %ge0 = icmp uge i8 %ch, 48
  %le9 = icmp ule i8 %ch, 57
  %is_digit = and i1 %ge0, %le9
  br i1 %is_digit, label %body_ck, label %done

done:                                             ; preds = %loop_body, %loop_hdr, %entry
  %final = load i64, ptr %result, align 4
  %final_sign = load i64, ptr %sign, align 4
  %mul_sign = mul i64 %final, %final_sign
  %valid_val = load i1, ptr %valid, align 1
  %ret_val = insertvalue { i64, i1 } undef, i64 %mul_sign, 0
  %ret_ok = insertvalue { i64, i1 } %ret_val, i1 %valid_val, 1
  ret { i64, i1 } %ret_ok

body_ck:                                          ; preds = %loop_body
  %cur = load i64, ptr %result, align 4
  %mul = mul i64 %cur, 10
  %dval = sub i8 %ch, 48
  %dval64 = zext i8 %dval to i64
  %add = add i64 %mul, %dval64
  store i64 %add, ptr %result, align 4
  store i1 true, ptr %valid, align 1
  br label %body_next

body_next:                                        ; preds = %body_ck
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %i, align 4
  br label %loop_hdr
}

define %__action_str @action_read_file(%__action_str %0) {
entry:
  %path_data = extractvalue %__action_str %0, 1
  %file = call ptr @fopen(ptr %path_data, ptr @.rf_mode)
  %rf_i64 = ptrtoint ptr %file to i64
  %rf_null = icmp eq i64 %rf_i64, 0
  br i1 %rf_null, label %fail, label %open_ok

open_ok:                                          ; preds = %entry
  %1 = call i32 @fseek(ptr %file, i64 0, i32 2)
  %size = call i64 @ftell(ptr %file)
  %2 = call i32 @fseek(ptr %file, i64 0, i32 0)
  %alc = add i64 %size, 1
  %buf = call ptr @malloc(i64 %alc)
  %3 = call i64 @fread(ptr %buf, i64 1, i64 %size, ptr %file)
  %null_gep = getelementptr i8, ptr %buf, i64 %size
  store i8 0, ptr %null_gep, align 1
  %4 = call i32 @fclose(ptr %file)
  %r1 = insertvalue %__action_str undef, i64 %size, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2

fail:                                             ; preds = %entry
  ret %__action_str zeroinitializer
}

define i1 @action_write_file(%__action_str %0, %__action_str %1) {
entry:
  %pdata = extractvalue %__action_str %0, 1
  %clen = extractvalue %__action_str %1, 0
  %cdata = extractvalue %__action_str %1, 1
  %file = call ptr @fopen(ptr %pdata, ptr @.wf_mode)
  %wf_i64 = ptrtoint ptr %file to i64
  %wf_null = icmp eq i64 %wf_i64, 0
  br i1 %wf_null, label %wf_fail, label %open_ok

open_ok:                                          ; preds = %entry
  %2 = call i64 @fwrite(ptr %cdata, i64 1, i64 %clen, ptr %file)
  %3 = call i32 @fclose(ptr %file)
  br label %wf_done

wf_fail:                                          ; preds = %entry
  br label %wf_done

wf_done:                                          ; preds = %open_ok, %wf_fail
  %wf_ok = phi i1 [ false, %wf_fail ], [ true, %open_ok ]
  ret i1 %wf_ok
}

define i1 @action_file_exists(%__action_str %0) {
entry:
  %pdata = extractvalue %__action_str %0, 1
  %file = call ptr @fopen(ptr %pdata, ptr @.fe_mode)
  %fe_i64 = ptrtoint ptr %file to i64
  %fe_null = icmp eq i64 %fe_i64, 0
  br i1 %fe_null, label %fe_done, label %exists_ok

exists_ok:                                        ; preds = %entry
  %1 = call i32 @fclose(ptr %file)
  br label %fe_done

fe_done:                                          ; preds = %exists_ok, %entry
  %fe_exists = phi i1 [ false, %entry ], [ true, %exists_ok ]
  ret i1 %fe_exists
}

define i1 @action_file_append(%__action_str %0, %__action_str %1) {
entry:
  %pdata = extractvalue %__action_str %0, 1
  %clen = extractvalue %__action_str %1, 0
  %cdata = extractvalue %__action_str %1, 1
  %file = call ptr @fopen(ptr %pdata, ptr @.fa_mode)
  %fa_i64 = ptrtoint ptr %file to i64
  %fa_null = icmp eq i64 %fa_i64, 0
  br i1 %fa_null, label %fa_fail, label %open_ok

open_ok:                                          ; preds = %entry
  %2 = call i64 @fwrite(ptr %cdata, i64 1, i64 %clen, ptr %file)
  %3 = call i32 @fclose(ptr %file)
  br label %fa_done

fa_fail:                                          ; preds = %entry
  br label %fa_done

fa_done:                                          ; preds = %open_ok, %fa_fail
  %fa_ok = phi i1 [ false, %fa_fail ], [ true, %open_ok ]
  ret i1 %fa_ok
}

define i1 @action_file_delete(%__action_str %0) {
entry:
  %pdata = extractvalue %__action_str %0, 1
  %ret = call i32 @remove(ptr %pdata)
  %fd_ok = icmp eq i32 %ret, 0
  ret i1 %fd_ok
}

define ptr @action_file_open(%__action_str %0, %__action_str %1) {
entry:
  %pdata = extractvalue %__action_str %0, 1
  %mdata = extractvalue %__action_str %1, 1
  %file = call ptr @fopen(ptr %pdata, ptr %mdata)
  ret ptr %file
}

define i32 @action_file_close(ptr %0) {
entry:
  %ret = call i32 @fclose(ptr %0)
  ret i32 %ret
}

declare i32 @feof(ptr)

define i1 @action_file_eof(ptr %0) {
entry:
  %ret = call i32 @feof(ptr %0)
  %is_eof = icmp ne i32 %ret, 0
  ret i1 %is_eof
}

define { i64, ptr, i1 } @action_file_read_line(ptr %0) {
entry:
  %buf = call ptr @malloc(i64 4096)
  %1 = call ptr @fgets(ptr %buf, i32 4096, ptr %0)
  %is_eof = icmp eq ptr %1, null
  br i1 %is_eof, label %eof, label %ok

eof:                                              ; preds = %entry
  br label %merge

ok:                                               ; preds = %entry
  %len = call i64 @strlen(ptr %buf)
  %last_idx = sub i64 %len, 1
  %last_ptr = getelementptr i8, ptr %buf, i64 %last_idx
  %last_ch = load i8, ptr %last_ptr, align 1
  %is_nl = icmp eq i8 %last_ch, 10
  %adj_len = select i1 %is_nl, i64 %last_idx, i64 %len
  %o_len = insertvalue { i64, ptr, i1 } undef, i64 %adj_len, 0
  %o_ptr = insertvalue { i64, ptr, i1 } %o_len, ptr %buf, 1
  %o_ok = insertvalue { i64, ptr, i1 } %o_ptr, i1 true, 2
  br label %merge

merge:                                            ; preds = %ok, %eof
  %frl_ret = phi { i64, ptr, i1 } [ zeroinitializer, %eof ], [ %o_ok, %ok ]
  ret { i64, ptr, i1 } %frl_ret
}

define { i64, ptr } @action_file_read_bytes(ptr %0, i64 %1) {
entry:
  %buf = call ptr @malloc(i64 %1)
  %read = call i64 @fread(ptr %buf, i64 1, i64 %1, ptr %0)
  %r_len = insertvalue { i64, ptr } undef, i64 %read, 0
  %r_ptr = insertvalue { i64, ptr } %r_len, ptr %buf, 1
  ret { i64, ptr } %r_ptr
}

define i1 @action_file_write_bytes(ptr %0, ptr %1, i64 %2) {
entry:
  %written = call i64 @fwrite(ptr %1, i64 1, i64 %2, ptr %0)
  %ok = icmp eq i64 %written, %2
  ret i1 %ok
}

define i1 @action_file_seek(ptr %0, i64 %1, i32 %2) {
entry:
  %ret = call i32 @fseek(ptr %0, i64 %1, i32 %2)
  %ok = icmp eq i32 %ret, 0
  ret i1 %ok
}

define i64 @action_file_tell(ptr %0) {
entry:
  %ret = call i64 @ftell(ptr %0)
  ret i64 %ret
}

declare i32 @fflush(ptr)

define i1 @action_file_flush(ptr %0) {
entry:
  %ret = call i32 @fflush(ptr %0)
  %ok = icmp eq i32 %ret, 0
  ret i1 %ok
}

define i64 @action_rand_int(i64 %0, i64 %1) {
entry:
  %old_seed = load i64, ptr @action_rand_seed, align 4
  %mul = mul i64 %old_seed, 1103515245
  %new_seed = add i64 %mul, 12345
  store i64 %new_seed, ptr @action_rand_seed, align 4
  %sub = sub i64 %1, %0
  %range1 = add i64 %sub, 1
  %pos = icmp sgt i64 %range1, 0
  %rem = urem i64 %new_seed, %range1
  %zero_range = icmp ule i64 %range1, 0
  %add = add i64 %0, %rem
  %result = select i1 %zero_range, i64 %0, i64 %add
  ret i64 %result
}

define double @action_rand_float() {
entry:
  %old_seed = load i64, ptr @action_rand_seed, align 4
  %mul = mul i64 %old_seed, 1103515245
  %new_seed = add i64 %mul, 12345
  store i64 %new_seed, ptr @action_rand_seed, align 4
  %masked = and i64 %new_seed, 9223372036854775807
  %f64 = uitofp i64 %masked to double
  %result = fdiv double %f64, 0x43E0000000000000
  ret double %result
}

define { ptr, i64, i64 } @action_string_split(%__action_str %0, %__action_str %1) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sdata = extractvalue %__action_str %0, 1
  %dlen = extractvalue %__action_str %1, 0
  %ddata = extractvalue %__action_str %1, 1
  %count = alloca i64, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %count, align 4
  store i64 0, ptr %i, align 4
  %dzero = icmp eq i64 %dlen, 0
  br i1 %dzero, label %cnt_done, label %cnt_hdr

cnt_hdr:                                          ; preds = %cnt_next, %cnt_ck, %entry
  %iv = load i64, ptr %i, align 4
  %end = add i64 %iv, %dlen
  %in_range = icmp ule i64 %end, %slen
  br i1 %in_range, label %cnt_body, label %cnt_done

cnt_body:                                         ; preds = %cnt_hdr
  %src = getelementptr i8, ptr %sdata, i64 %iv
  %mc = call i32 @memcmp(ptr %src, ptr %ddata, i64 %dlen)
  %match = icmp eq i32 %mc, 0
  br i1 %match, label %cnt_ck, label %cnt_next

cnt_ck:                                           ; preds = %cnt_body
  %cur = load i64, ptr %count, align 4
  %nc = add i64 %cur, 1
  store i64 %nc, ptr %count, align 4
  %ni = add i64 %iv, %dlen
  store i64 %ni, ptr %i, align 4
  br label %cnt_hdr

cnt_next:                                         ; preds = %cnt_body
  %ni2 = add i64 %iv, 1
  store i64 %ni2, ptr %i, align 4
  br label %cnt_hdr

cnt_done:                                         ; preds = %cnt_hdr, %entry
  %final_cnt = load i64, ptr %count, align 4
  %cap = add i64 %final_cnt, 1
  %list_alloc = call ptr @malloc(i64 8)
  %dsize = mul i64 %cap, 16
  %data = call ptr @malloc(i64 %dsize)
  %lr1 = insertvalue { ptr, i64, i64 } undef, ptr %data, 0
  %lr2 = insertvalue { ptr, i64, i64 } %lr1, i64 0, 1
  %lr3 = insertvalue { ptr, i64, i64 } %lr2, i64 %cap, 2
  %list_ptr = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %lr3, ptr %list_ptr, align 8
  %last = alloca i64, align 8
  store i64 0, ptr %i, align 4
  store i64 0, ptr %last, align 4
  br i1 %dzero, label %fill_last, label %fill_hdr

fill_hdr:                                         ; preds = %fill_next, %fill_push, %cnt_done
  %iv2 = load i64, ptr %i, align 4
  %end2 = add i64 %iv2, %dlen
  %in2 = icmp ule i64 %end2, %slen
  br i1 %in2, label %fill_body, label %fill_last

fill_body:                                        ; preds = %fill_hdr
  %src2 = getelementptr i8, ptr %sdata, i64 %iv2
  %mc2 = call i32 @memcmp(ptr %src2, ptr %ddata, i64 %dlen)
  %m2 = icmp eq i32 %mc2, 0
  br i1 %m2, label %fill_ck2, label %fill_next

fill_ck2:                                         ; preds = %fill_body
  %last_v = load i64, ptr %last, align 4
  %seg_len = sub i64 %iv2, %last_v
  %salc = add i64 %seg_len, 1
  %sbuf = call ptr @malloc(i64 %salc)
  %ssrc = getelementptr i8, ptr %sdata, i64 %last_v
  %2 = call ptr @memcpy(ptr %sbuf, ptr %ssrc, i64 %seg_len)
  %snull = getelementptr i8, ptr %sbuf, i64 %seg_len
  store i8 0, ptr %snull, align 1
  br label %fill_push

fill_push:                                        ; preds = %fill_ck2
  %fat1 = insertvalue %__action_str undef, i64 %seg_len, 0
  %fat2 = insertvalue %__action_str %fat1, ptr %sbuf, 1
  %ll = load { ptr, i64, i64 }, ptr %list_ptr, align 8
  %llen = extractvalue { ptr, i64, i64 } %ll, 1
  %ldata = extractvalue { ptr, i64, i64 } %ll, 0
  %offset = mul i64 %llen, 16
  %dst = getelementptr i8, ptr %ldata, i64 %offset
  %ftag = extractvalue %__action_str %fat2, 0
  store i64 %ftag, ptr %dst, align 4
  %fp = extractvalue %__action_str %fat2, 1
  %off1 = add i64 %offset, 8
  %pp = getelementptr i8, ptr %ldata, i64 %off1
  %fp_i64 = ptrtoint ptr %fp to i64
  store i64 %fp_i64, ptr %pp, align 4
  %nlen = add i64 %llen, 1
  %nl1 = insertvalue { ptr, i64, i64 } undef, ptr %ldata, 0
  %nl2 = insertvalue { ptr, i64, i64 } %nl1, i64 %nlen, 1
  %nl3 = insertvalue { ptr, i64, i64 } %nl2, i64 %cap, 2
  store { ptr, i64, i64 } %nl3, ptr %list_ptr, align 8
  %nlast = add i64 %iv2, %dlen
  store i64 %nlast, ptr %i, align 4
  store i64 %nlast, ptr %last, align 4
  br label %fill_hdr

fill_next:                                        ; preds = %fill_body
  %ni3 = add i64 %iv2, 1
  store i64 %ni3, ptr %i, align 4
  br label %fill_hdr

fill_last:                                        ; preds = %fill_hdr, %cnt_done
  %last_v2 = load i64, ptr %last, align 4
  %seg_len2 = sub i64 %slen, %last_v2
  %salc2 = add i64 %seg_len2, 1
  %sbuf2 = call ptr @malloc(i64 %salc2)
  %ssrc2 = getelementptr i8, ptr %sdata, i64 %last_v2
  %3 = call ptr @memcpy(ptr %sbuf2, ptr %ssrc2, i64 %seg_len2)
  %snull2 = getelementptr i8, ptr %sbuf2, i64 %seg_len2
  store i8 0, ptr %snull2, align 1
  %fat1b = insertvalue %__action_str undef, i64 %seg_len2, 0
  %fat2b = insertvalue %__action_str %fat1b, ptr %sbuf2, 1
  %ll2 = load { ptr, i64, i64 }, ptr %list_ptr, align 8
  %llen2 = extractvalue { ptr, i64, i64 } %ll2, 1
  %ldata2 = extractvalue { ptr, i64, i64 } %ll2, 0
  %offset2 = mul i64 %llen2, 16
  %dst2 = getelementptr i8, ptr %ldata2, i64 %offset2
  %ftag2 = extractvalue %__action_str %fat2b, 0
  store i64 %ftag2, ptr %dst2, align 4
  %fp2 = extractvalue %__action_str %fat2b, 1
  %off1b = add i64 %offset2, 8
  %pp2 = getelementptr i8, ptr %ldata2, i64 %off1b
  %fp2_i64 = ptrtoint ptr %fp2 to i64
  store i64 %fp2_i64, ptr %pp2, align 4
  %nlen2 = add i64 %llen2, 1
  %nl1b = insertvalue { ptr, i64, i64 } undef, ptr %ldata2, 0
  %nl2b = insertvalue { ptr, i64, i64 } %nl1b, i64 %nlen2, 1
  %nl3b = insertvalue { ptr, i64, i64 } %nl2b, i64 %cap, 2
  store { ptr, i64, i64 } %nl3b, ptr %list_ptr, align 8
  br label %fill_done

fill_done:                                        ; preds = %fill_last
  %result = load { ptr, i64, i64 }, ptr %list_ptr, align 8
  ret { ptr, i64, i64 } %result
}

define %__action_str @action_string_join({ ptr, i64, i64 } %0, %__action_str %1) {
entry:
  %ldata = extractvalue { ptr, i64, i64 } %0, 0
  %llen = extractvalue { ptr, i64, i64 } %0, 1
  %dlen = extractvalue %__action_str %1, 0
  %ddata = extractvalue %__action_str %1, 1
  %total = alloca i64, align 8
  store i64 0, ptr %total, align 4
  %ji = alloca i64, align 8
  store i64 0, ptr %ji, align 4
  br label %hdr

hdr:                                              ; preds = %body, %entry
  %iv = load i64, ptr %ji, align 4
  %more = icmp ult i64 %iv, %llen
  br i1 %more, label %body, label %after

body:                                             ; preds = %hdr
  %off = mul i64 %iv, 16
  %ep = getelementptr i8, ptr %ldata, i64 %off
  %sslen = load i64, ptr %ep, align 4
  %cur = load i64, ptr %total, align 4
  %add = add i64 %cur, %sslen
  %ivp1 = add i64 %iv, 1
  %is_last = icmp eq i64 %ivp1, %llen
  %with_delim = add i64 %sslen, %dlen
  %delta = select i1 %is_last, i64 %sslen, i64 %with_delim
  %new_total = add i64 %cur, %delta
  store i64 %new_total, ptr %total, align 4
  %niv = add i64 %iv, 1
  store i64 %niv, ptr %ji, align 4
  br label %hdr

after:                                            ; preds = %hdr
  %final_total = load i64, ptr %total, align 4
  %jalc = add i64 %final_total, 1
  %buf = call ptr @malloc(i64 %jalc)
  %wpos = alloca i64, align 8
  store i64 0, ptr %ji, align 4
  store i64 0, ptr %wpos, align 4
  br label %chdr

chdr:                                             ; preds = %cnext, %after
  %civ = load i64, ptr %ji, align 4
  %cmore = icmp ult i64 %civ, %llen
  br i1 %cmore, label %cbody, label %cdone

cbody:                                            ; preds = %chdr
  %coff = mul i64 %civ, 16
  %cep = getelementptr i8, ptr %ldata, i64 %coff
  %csslen = load i64, ptr %cep, align 4
  %coff1 = add i64 %coff, 8
  %cpp = getelementptr i8, ptr %ldata, i64 %coff1
  %cpval = load i64, ptr %cpp, align 4
  %cp = inttoptr i64 %cpval to ptr
  %cwp = load i64, ptr %wpos, align 4
  %cdst = getelementptr i8, ptr %buf, i64 %cwp
  %2 = call ptr @memcpy(ptr %cdst, ptr %cp, i64 %csslen)
  %nwp = add i64 %cwp, %csslen
  store i64 %nwp, ptr %wpos, align 4
  %civp1 = add i64 %civ, 1
  %cis_last = icmp eq i64 %civp1, %llen
  br i1 %cis_last, label %cnext, label %cdel

cdone:                                            ; preds = %chdr
  %fwp = load i64, ptr %wpos, align 4
  %nullp = getelementptr i8, ptr %buf, i64 %fwp
  store i8 0, ptr %nullp, align 1
  %r1 = insertvalue %__action_str undef, i64 %fwp, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2

cdel:                                             ; preds = %cbody
  %cwp2 = load i64, ptr %wpos, align 4
  %cdst2 = getelementptr i8, ptr %buf, i64 %cwp2
  %3 = call ptr @memcpy(ptr %cdst2, ptr %ddata, i64 %dlen)
  %nwp2 = add i64 %cwp2, %dlen
  store i64 %nwp2, ptr %wpos, align 4
  br label %cnext

cnext:                                            ; preds = %cdel, %cbody
  %cniv = add i64 %civ, 1
  store i64 %cniv, ptr %ji, align 4
  br label %chdr
}

define %__action_str @action_string_replace(%__action_str %0, %__action_str %1, %__action_str %2) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sdata = extractvalue %__action_str %0, 1
  %flen = extractvalue %__action_str %1, 0
  %fdata = extractvalue %__action_str %1, 1
  %tlen = extractvalue %__action_str %2, 0
  %tdata = extractvalue %__action_str %2, 1
  %fzero = icmp eq i64 %flen, 0
  br i1 %fzero, label %copy_ret, label %have_from

have_from:                                        ; preds = %entry
  %ri = alloca i64, align 8
  %rlast = alloca i64, align 8
  %rcount = alloca i64, align 8
  store i64 0, ptr %ri, align 4
  store i64 0, ptr %rlast, align 4
  store i64 0, ptr %rcount, align 4
  br label %hdr

copy_ret:                                         ; preds = %entry
  %calc = add i64 %slen, 1
  %cbuf = call ptr @malloc(i64 %calc)
  %3 = call ptr @memcpy(ptr %cbuf, ptr %sdata, i64 %slen)
  %cnull = getelementptr i8, ptr %cbuf, i64 %slen
  store i8 0, ptr %cnull, align 1
  %cr1 = insertvalue %__action_str undef, i64 %slen, 0
  %cr2 = insertvalue %__action_str %cr1, ptr %cbuf, 1
  ret %__action_str %cr2

hdr:                                              ; preds = %nxt, %ck, %have_from
  %riv = load i64, ptr %ri, align 4
  %end = add i64 %riv, %flen
  %ok = icmp ule i64 %end, %slen
  br i1 %ok, label %body, label %build

body:                                             ; preds = %hdr
  %rsrc = getelementptr i8, ptr %sdata, i64 %riv
  %rmc = call i32 @memcmp(ptr %rsrc, ptr %fdata, i64 %flen)
  %rm = icmp eq i32 %rmc, 0
  br i1 %rm, label %ck, label %nxt

ck:                                               ; preds = %body
  %rc = load i64, ptr %rcount, align 4
  %nc = add i64 %rc, 1
  store i64 %nc, ptr %rcount, align 4
  %nri = add i64 %riv, %flen
  store i64 %nri, ptr %ri, align 4
  br label %hdr

nxt:                                              ; preds = %body
  %nri2 = add i64 %riv, 1
  store i64 %nri2, ptr %ri, align 4
  br label %hdr

build:                                            ; preds = %hdr
  %fc = load i64, ptr %rcount, align 4
  %diff = sub i64 %tlen, %flen
  %extra = mul i64 %fc, %diff
  %nlen = add i64 %slen, %extra
  %nalc = add i64 %nlen, 1
  %nbuf = call ptr @malloc(i64 %nalc)
  store i64 0, ptr %ri, align 4
  store i64 0, ptr %rlast, align 4
  %wpos = alloca i64, align 8
  store i64 0, ptr %wpos, align 4
  br label %bhdr

bhdr:                                             ; preds = %bnxt, %bck, %build
  %briv = load i64, ptr %ri, align 4
  %bend = add i64 %briv, %flen
  %bok = icmp ule i64 %bend, %slen
  br i1 %bok, label %bbody, label %bfinal

bbody:                                            ; preds = %bhdr
  %brsrc = getelementptr i8, ptr %sdata, i64 %briv
  %bmc = call i32 @memcmp(ptr %brsrc, ptr %fdata, i64 %flen)
  %bm = icmp eq i32 %bmc, 0
  br i1 %bm, label %bck, label %bnxt

bck:                                              ; preds = %bbody
  %blast = load i64, ptr %rlast, align 4
  %bgap = sub i64 %briv, %blast
  %bwp = load i64, ptr %wpos, align 4
  %bgsrc = getelementptr i8, ptr %sdata, i64 %blast
  %bgdst = getelementptr i8, ptr %nbuf, i64 %bwp
  %4 = call ptr @memcpy(ptr %bgdst, ptr %bgsrc, i64 %bgap)
  %bnwp1 = add i64 %bwp, %bgap
  %brdst = getelementptr i8, ptr %nbuf, i64 %bnwp1
  %5 = call ptr @memcpy(ptr %brdst, ptr %tdata, i64 %tlen)
  %bnwp2 = add i64 %bnwp1, %tlen
  store i64 %bnwp2, ptr %wpos, align 4
  %bnri = add i64 %briv, %flen
  store i64 %bnri, ptr %ri, align 4
  store i64 %bnri, ptr %rlast, align 4
  br label %bhdr

bnxt:                                             ; preds = %bbody
  %bnri2 = add i64 %briv, 1
  store i64 %bnri2, ptr %ri, align 4
  br label %bhdr

bfinal:                                           ; preds = %bhdr
  %blast2 = load i64, ptr %rlast, align 4
  %brem = sub i64 %slen, %blast2
  %bwp2 = load i64, ptr %wpos, align 4
  %brsrc2 = getelementptr i8, ptr %sdata, i64 %blast2
  %brdst2 = getelementptr i8, ptr %nbuf, i64 %bwp2
  %6 = call ptr @memcpy(ptr %brdst2, ptr %brsrc2, i64 %brem)
  %bnwp3 = add i64 %bwp2, %brem
  br label %bdone

bdone:                                            ; preds = %bfinal
  %fwpos = load i64, ptr %wpos, align 4
  %bnull = getelementptr i8, ptr %nbuf, i64 %fwpos
  store i8 0, ptr %bnull, align 1
  %rr1 = insertvalue %__action_str undef, i64 %fwpos, 0
  %rr2 = insertvalue %__action_str %rr1, ptr %nbuf, 1
  ret %__action_str %rr2
}

define i1 @action_string_contains(%__action_str %0, %__action_str %1) {
entry:
  %hlen = extractvalue %__action_str %0, 0
  %hptr = extractvalue %__action_str %0, 1
  %nlen = extractvalue %__action_str %1, 0
  %nptr = extractvalue %__action_str %1, 1
  %nempty = icmp eq i64 %nlen, 0
  %lenok = icmp sle i64 %nlen, %hlen
  %not_empty = xor i1 %nempty, true
  %can_search = and i1 %lenok, %not_empty
  %max = sub i64 %hlen, %nlen
  br label %sc_loop

sc_loop:                                          ; preds = %sc_mismatch, %entry
  %sc_i = phi i64 [ 0, %entry ], [ %inext, %sc_mismatch ]
  br label %sc_jloop

sc_found:                                         ; preds = %sc_match
  ret i1 true

sc_notfound:                                      ; preds = %sc_mismatch
  ret i1 false

sc_jloop:                                         ; preds = %sc_match, %sc_loop
  %sc_j = phi i64 [ 0, %sc_loop ], [ %jnext, %sc_match ]
  %hidx = add i64 %sc_i, %sc_j
  %hp = getelementptr i8, ptr %hptr, i64 %hidx
  %hc = load i8, ptr %hp, align 1
  %np = getelementptr i8, ptr %nptr, i64 %sc_j
  %nc = load i8, ptr %np, align 1
  %char_match = icmp eq i8 %hc, %nc
  %jnext = add i64 %sc_j, 1
  %jdone = icmp sge i64 %jnext, %nlen
  br i1 %char_match, label %sc_match, label %sc_mismatch

sc_match:                                         ; preds = %sc_jloop
  br i1 %jdone, label %sc_found, label %sc_jloop

sc_mismatch:                                      ; preds = %sc_jloop
  %inext = add i64 %sc_i, 1
  %idone = icmp sgt i64 %inext, %max
  br i1 %idone, label %sc_notfound, label %sc_loop
}

define %__action_str @action_string_repeat(%__action_str %0, i64 %1) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sptr = extractvalue %__action_str %0, 1
  %total = mul i64 %slen, %1
  %buf = call ptr @malloc(i64 %total)
  br label %sr_loop

sr_loop:                                          ; preds = %sr_loop, %entry
  %sr_i = phi i64 [ 0, %entry ], [ %sri_next, %sr_loop ]
  %offset = mul i64 %sr_i, %slen
  %dst = getelementptr i8, ptr %buf, i64 %offset
  %2 = call ptr @memcpy(ptr %dst, ptr %sptr, i64 %slen)
  %sri_next = add i64 %sr_i, 1
  %srdone = icmp sge i64 %sri_next, %1
  br i1 %srdone, label %sr_done, label %sr_loop

sr_done:                                          ; preds = %sr_loop
  %r1 = insertvalue %__action_str undef, i64 %total, 0
  %r2 = insertvalue %__action_str %r1, ptr %buf, 1
  ret %__action_str %r2
}

define %__action_str @action_string_trim_start(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %ptr = extractvalue %__action_str %0, 1
  br label %ts_loop

ts_loop:                                          ; preds = %ts_loop, %entry
  %ts_i = phi i64 [ 0, %entry ], [ %ts_inext, %ts_loop ]
  %cp = getelementptr i8, ptr %ptr, i64 %ts_i
  %c = load i8, ptr %cp, align 1
  %is_space = icmp eq i8 %c, 32
  %is_tab = icmp eq i8 %c, 9
  %is_nl = icmp eq i8 %c, 10
  %is_cr = icmp eq i8 %c, 13
  %ws1 = or i1 %is_space, %is_tab
  %ws2 = or i1 %is_nl, %is_cr
  %is_ws = or i1 %ws1, %ws2
  %ts_inext = add i64 %ts_i, 1
  %at_end = icmp sge i64 %ts_inext, %len
  %not_ws = xor i1 %is_ws, true
  %stop = or i1 %at_end, %not_ws
  br i1 %stop, label %ts_done, label %ts_loop

ts_done:                                          ; preds = %ts_loop
  %ts_start = phi i64 [ %ts_i, %ts_loop ]
  %new_len = sub i64 %len, %ts_start
  %nptr = getelementptr i8, ptr %ptr, i64 %ts_start
  %r1 = insertvalue %__action_str undef, i64 %new_len, 0
  %r2 = insertvalue %__action_str %r1, ptr %nptr, 1
  ret %__action_str %r2
}

define %__action_str @action_string_trim_end(%__action_str %0) {
entry:
  %len = extractvalue %__action_str %0, 0
  %ptr = extractvalue %__action_str %0, 1
  %last = sub i64 %len, 1
  br label %te_loop

te_loop:                                          ; preds = %te_loop, %entry
  %te_i = phi i64 [ %last, %entry ], [ %te_inext, %te_loop ]
  %cp = getelementptr i8, ptr %ptr, i64 %te_i
  %c = load i8, ptr %cp, align 1
  %is_space = icmp eq i8 %c, 32
  %is_tab = icmp eq i8 %c, 9
  %is_nl = icmp eq i8 %c, 10
  %is_cr = icmp eq i8 %c, 13
  %ws1 = or i1 %is_space, %is_tab
  %ws2 = or i1 %is_nl, %is_cr
  %is_ws = or i1 %ws1, %ws2
  %te_inext = sub i64 %te_i, 1
  %neg = icmp slt i64 %te_inext, 0
  %not_ws = xor i1 %is_ws, true
  %stop = or i1 %neg, %not_ws
  br i1 %stop, label %te_done, label %te_loop

te_done:                                          ; preds = %te_loop
  %neg_check = icmp slt i64 %te_i, 0
  %fcp = getelementptr i8, ptr %ptr, i64 %te_i
  %fc = load i8, ptr %fcp, align 1
  %1 = icmp eq i8 %fc, 32
  %2 = icmp eq i8 %fc, 9
  %3 = or i1 %1, %2
  %4 = icmp eq i8 %fc, 10
  %5 = icmp eq i8 %fc, 13
  %6 = or i1 %4, %5
  %fws = or i1 %3, %6
  %plus1 = add i64 %te_i, 1
  %new_len = select i1 %fws, i64 0, i64 %plus1
  %r1 = insertvalue %__action_str undef, i64 %new_len, 0
  %r2 = insertvalue %__action_str %r1, ptr %ptr, 1
  ret %__action_str %r2
}

define { ptr, i64, i64 } @action_list_tail({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %empty = icmp eq i64 %len, 0
  %empty_or_one = icmp sle i64 %len, 1
  br i1 %empty_or_one, label %empty_ret, label %do

do:                                               ; preds = %entry
  %nlen = sub i64 %len, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 %nlen)
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 1, ptr %i, align 4
  br label %loop

empty_ret:                                        ; preds = %entry
  %2 = call { ptr, i64, i64 } @action_list_create(i64 0)
  ret { ptr, i64, i64 } %2

loop:                                             ; preds = %body, %do
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %i1
  %fv = load %__action_str, ptr %ep, align 8
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %fv)
  store { ptr, i64, i64 } %3, ptr %newacc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv
}

define { ptr, i64, i64 } @action_list_zip({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %alen = extractvalue { ptr, i64, i64 } %0, 1
  %blen = extractvalue { ptr, i64, i64 } %1, 1
  %altb = icmp slt i64 %alen, %blen
  %min = select i1 %altb, i64 %alen, i64 %blen
  %2 = call { ptr, i64, i64 } @action_list_create(i64 %min)
  %adata = extractvalue { ptr, i64, i64 } %0, 0
  %bdata = extractvalue { ptr, i64, i64 } %1, 0
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %2, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %min
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %afat = getelementptr %__action_str, ptr %adata, i64 %i1
  %bfat = getelementptr %__action_str, ptr %bdata, i64 %i1
  %av = load %__action_str, ptr %afat, align 8
  %bv = load %__action_str, ptr %bfat, align 8
  %tup = call ptr @malloc(i64 32)
  %ta = getelementptr inbounds nuw { %__action_str, %__action_str }, ptr %tup, i32 0, i32 0
  %tb = getelementptr inbounds nuw { %__action_str, %__action_str }, ptr %tup, i32 0, i32 1
  store %__action_str %av, ptr %ta, align 8
  store %__action_str %bv, ptr %tb, align 8
  %data = insertvalue %__action_str { i64 5, ptr undef }, ptr %tup, 1
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %data)
  store { ptr, i64, i64 } %3, ptr %newacc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv
}

define { ptr, i64, i64 } @action_list_init({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %empty_ret, label %do

do:                                               ; preds = %entry
  %nlen = sub i64 %len, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 %nlen)
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

empty_ret:                                        ; preds = %entry
  %2 = call { ptr, i64, i64 } @action_list_create(i64 0)
  ret { ptr, i64, i64 } %2

loop:                                             ; preds = %body, %do
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %nlen
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %i1
  %fv = load %__action_str, ptr %ep, align 8
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %fv)
  store { ptr, i64, i64 } %3, ptr %newacc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv
}

define %__action_str @action_list_last({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %none, label %has

has:                                              ; preds = %entry
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %last_idx = sub i64 %len, 1
  %elem_ptr = getelementptr %__action_str, ptr %data, i64 %last_idx
  %val = load %__action_str, ptr %elem_ptr, align 8
  ret %__action_str %val

none:                                             ; preds = %entry
  ret %__action_str zeroinitializer
}

define { ptr, i64, i64 } @action_string_chars(%__action_str %0) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sptr = extractvalue %__action_str %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 %slen)
  %list_acc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %list_acc, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %slen
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %cp = getelementptr i8, ptr %sptr, i64 %i1
  %c = load i8, ptr %cp, align 1
  %salloc = call ptr @malloc(i64 1)
  store i8 %c, ptr %salloc, align 1
  %data = insertvalue %__action_str { i64 1, ptr undef }, ptr %salloc, 1
  %cur = load { ptr, i64, i64 }, ptr %list_acc, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %data)
  store { ptr, i64, i64 } %2, ptr %list_acc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %list_acc, align 8
  ret { ptr, i64, i64 } %rv
}

define { ptr, i64, i64 } @action_list_with_index({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %1 = call { ptr, i64, i64 } @action_list_create(i64 %len)
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %i1
  %ev = load %__action_str, ptr %ep, align 8
  %tup = call ptr @malloc(i64 24)
  %ti = getelementptr inbounds nuw { i64, %__action_str }, ptr %tup, i32 0, i32 0
  %te = getelementptr inbounds nuw { i64, %__action_str }, ptr %tup, i32 0, i32 1
  store i64 %i1, ptr %ti, align 4
  store %__action_str %ev, ptr %te, align 8
  %data2 = insertvalue %__action_str { i64 5, ptr undef }, ptr %tup, 1
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %data2)
  store { ptr, i64, i64 } %2, ptr %newacc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv
}

define { ptr, i64, i64 } @action_list_unique({ ptr, i64, i64 } %0) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %1 = call { ptr, i64, i64 } @action_list_create(i64 0)
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %skip, %entry
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %i1
  %ev = load %__action_str, ptr %ep, align 8
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %2 = call i1 @action_list_contains({ ptr, i64, i64 } %cur, %__action_str %ev)
  br i1 %2, label %skip, label %push

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv

push:                                             ; preds = %body
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %ev)
  store { ptr, i64, i64 } %3, ptr %newacc, align 8
  br label %skip

skip:                                             ; preds = %push, %body
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop
}

define { ptr, i64, i64 } @action_list_slice({ ptr, i64, i64 } %0, i64 %1, i64 %2) {
entry:
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %sneg = icmp slt i64 %1, 0
  %sclamp = select i1 %sneg, i64 0, i64 %1
  %sgt = icmp sgt i64 %sclamp, %len
  %sfinal = select i1 %sgt, i64 %len, i64 %sclamp
  %eneg = icmp slt i64 %2, 0
  %eclamp = select i1 %eneg, i64 0, i64 %2
  %egt = icmp sgt i64 %eclamp, %len
  %efinal = select i1 %egt, i64 %len, i64 %eclamp
  %rlen = sub i64 %efinal, %sfinal
  %rneg = icmp slt i64 %rlen, 0
  %rlenf = select i1 %rneg, i64 0, i64 %rlen
  %3 = call { ptr, i64, i64 } @action_list_create(i64 %rlenf)
  %newacc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %3, ptr %newacc, align 8
  %i = alloca i64, align 8
  store i64 %sfinal, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %i1 = load i64, ptr %i, align 4
  %cond = icmp slt i64 %i1, %efinal
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %i1
  %ev = load %__action_str, ptr %ep, align 8
  %cur = load { ptr, i64, i64 }, ptr %newacc, align 8
  %4 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur, %__action_str %ev)
  store { ptr, i64, i64 } %4, ptr %newacc, align 8
  %ni = add i64 %i1, 1
  store i64 %ni, ptr %i, align 4
  br label %loop

done:                                             ; preds = %loop
  %rv = load { ptr, i64, i64 }, ptr %newacc, align 8
  ret { ptr, i64, i64 } %rv
}

define { ptr, i64, i64 } @action_string_split_lines(%__action_str %0) {
entry:
  %slen = extractvalue %__action_str %0, 0
  %sptr = extractvalue %__action_str %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 0)
  %list_acc = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %list_acc, align 8
  %start = alloca i64, align 8
  %i = alloca i64, align 8
  store i64 0, ptr %start, align 4
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %cont, %entry
  %sl_i = load i64, ptr %i, align 4
  %cond = icmp sle i64 %sl_i, %slen
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %atend = icmp eq i64 %sl_i, %slen
  %cp = getelementptr i8, ptr %sptr, i64 %sl_i
  %c = load i8, ptr %cp, align 1
  %isnl = icmp eq i8 %c, 10
  %iscr = icmp eq i8 %c, 13
  %2 = or i1 %isnl, %iscr
  %split = or i1 %atend, %2
  br i1 %split, label %extract, label %cont

done:                                             ; preds = %loop
  %result = load { ptr, i64, i64 }, ptr %list_acc, align 8
  ret { ptr, i64, i64 } %result

cont:                                             ; preds = %extract, %body
  %i2 = load i64, ptr %i, align 4
  %inext = add i64 %i2, 1
  store i64 %inext, ptr %i, align 4
  br label %loop

extract:                                          ; preds = %body
  %slstart = load i64, ptr %start, align 4
  %seg_len = sub i64 %sl_i, %slstart
  %segp = getelementptr i8, ptr %sptr, i64 %slstart
  %nexti = add i64 %sl_i, 1
  %seg = call ptr @malloc(i64 %seg_len)
  %3 = call ptr @memcpy(ptr %seg, ptr %segp, i64 %seg_len)
  %data = insertvalue %__action_str { i64 1, ptr undef }, ptr %seg, 1
  %cur_list = load { ptr, i64, i64 }, ptr %list_acc, align 8
  %4 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cur_list, %__action_str %data)
  store { ptr, i64, i64 } %4, ptr %list_acc, align 8
  store i64 %nexti, ptr %start, align 4
  br label %cont
}

define i64 @action_string_index_of(%__action_str %0, %__action_str %1) {
entry:
  %hlen = extractvalue %__action_str %0, 0
  %hptr = extractvalue %__action_str %0, 1
  %nlen = extractvalue %__action_str %1, 0
  %nptr = extractvalue %__action_str %1, 1
  %nempty = icmp eq i64 %nlen, 0
  %nok = icmp sle i64 %nlen, %hlen
  %2 = xor i1 %nempty, true
  %3 = and i1 %nok, %2
  %max = sub i64 %hlen, %nlen
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %oloop

oloop:                                            ; preds = %next, %entry
  %iv = load i64, ptr %i, align 4
  %cond = icmp sle i64 %iv, %max
  br i1 %cond, label %obody, label %notfound

obody:                                            ; preds = %oloop
  %hp = getelementptr i8, ptr %hptr, i64 %iv
  %eq = call i32 @memcmp(ptr %hp, ptr %nptr, i64 %nlen)
  %match = icmp eq i32 %eq, 0
  br i1 %match, label %match1, label %next

notfound:                                         ; preds = %oloop
  ret i64 -1

match1:                                           ; preds = %obody
  ret i64 %iv

next:                                             ; preds = %obody
  %nexti = add i64 %iv, 1
  store i64 %nexti, ptr %i, align 4
  br label %oloop
}

define { ptr, i64, i64 } @action_list_flatten({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %fl_res = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %fl_res, align 8
  %fl_oi = alloca i64, align 8
  store i64 0, ptr %fl_oi, align 4
  br label %oloop

oloop:                                            ; preds = %push_next, %entry
  %oi = load i64, ptr %fl_oi, align 4
  %ocond = icmp slt i64 %oi, %len
  br i1 %ocond, label %obody, label %odone

obody:                                            ; preds = %oloop
  %ep = getelementptr %__action_str, ptr %data, i64 %oi
  %elem = load %__action_str, ptr %ep, align 8
  %etag = extractvalue %__action_str %elem, 0
  %islist = icmp eq i64 %etag, 6
  br i1 %islist, label %push_flat, label %push_direct

odone:                                            ; preds = %oloop
  %fl_res1 = load { ptr, i64, i64 }, ptr %fl_res, align 8
  ret { ptr, i64, i64 } %fl_res1

push_flat:                                        ; preds = %obody
  %edata = extractvalue %__action_str %elem, 1
  %inner = load { ptr, i64, i64 }, ptr %edata, align 8
  %idata = extractvalue { ptr, i64, i64 } %inner, 0
  %ilen = extractvalue { ptr, i64, i64 } %inner, 1
  %fl_ii = alloca i64, align 8
  store i64 0, ptr %fl_ii, align 4
  br label %iloop

push_direct:                                      ; preds = %obody
  %cl2 = load { ptr, i64, i64 }, ptr %fl_res, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl2, %__action_str %elem)
  store { ptr, i64, i64 } %2, ptr %fl_res, align 8
  br label %push_next

push_next:                                        ; preds = %push_direct, %idone
  %oiinc = add i64 %oi, 1
  store i64 %oiinc, ptr %fl_oi, align 4
  br label %oloop

iloop:                                            ; preds = %ibody, %push_flat
  %ii = load i64, ptr %fl_ii, align 4
  %icond = icmp slt i64 %ii, %ilen
  br i1 %icond, label %ibody, label %idone

ibody:                                            ; preds = %iloop
  %iep = getelementptr %__action_str, ptr %idata, i64 %ii
  %ie = load %__action_str, ptr %iep, align 8
  %cl = load { ptr, i64, i64 }, ptr %fl_res, align 8
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %ie)
  store { ptr, i64, i64 } %3, ptr %fl_res, align 8
  %iiinc = add i64 %ii, 1
  store i64 %iiinc, ptr %fl_ii, align 4
  br label %iloop

idone:                                            ; preds = %iloop
  br label %push_next
}

define { ptr, i64, i64 } @action_list_split_at({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %cl = icmp slt i64 %1, 0
  %idx0 = select i1 %cl, i64 0, i64 %1
  %cl2 = icmp sgt i64 %idx0, %len
  %idx_safe = select i1 %cl2, i64 %len, i64 %idx0
  %2 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %sa_a1 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %2, ptr %sa_a1, align 8
  %3 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %sa_a2 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %3, ptr %sa_a2, align 8
  %sa_i = alloca i64, align 8
  store i64 0, ptr %sa_i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %iv = load i64, ptr %sa_i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %iv
  %ev = load %__action_str, ptr %ep, align 8
  %before = icmp slt i64 %iv, %idx_safe
  %l1 = load { ptr, i64, i64 }, ptr %sa_a1, align 8
  %l2 = load { ptr, i64, i64 }, ptr %sa_a2, align 8
  %4 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %l1, %__action_str %ev)
  %5 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %l2, %__action_str %ev)
  %l1s = select i1 %before, { ptr, i64, i64 } %4, { ptr, i64, i64 } %l1
  %l2s = select i1 %before, { ptr, i64, i64 } %l2, { ptr, i64, i64 } %5
  store { ptr, i64, i64 } %l1s, ptr %sa_a1, align 8
  store { ptr, i64, i64 } %l2s, ptr %sa_a2, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %sa_i, align 4
  br label %loop

done:                                             ; preds = %loop
  %sa_m = call ptr @malloc(i64 16)
  %l1f = load { ptr, i64, i64 }, ptr %sa_a1, align 8
  %l1p = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %l1f, ptr %l1p, align 8
  %v1 = insertvalue %__action_str { i64 6, ptr undef }, ptr %l1p, 1
  store %__action_str %v1, ptr %sa_m, align 8
  %s2 = getelementptr %__action_str, ptr %sa_m, i64 1
  %l2f = load { ptr, i64, i64 }, ptr %sa_a2, align 8
  %l2p = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %l2f, ptr %l2p, align 8
  %v2 = insertvalue %__action_str { i64 6, ptr undef }, ptr %l2p, 1
  store %__action_str %v2, ptr %s2, align 8
  %d = insertvalue { ptr, i64, i64 } undef, ptr %sa_m, 0
  %l = insertvalue { ptr, i64, i64 } %d, i64 2, 1
  %c = insertvalue { ptr, i64, i64 } %l, i64 2, 2
  ret { ptr, i64, i64 } %c
}

define { ptr, i64, i64 } @action_list_chunks({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %cz = icmp slt i64 %1, 1
  %csafe = select i1 %cz, i64 1, i64 %1
  %2 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %ch_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %2, ptr %ch_ra, align 8
  %ch_i = alloca i64, align 8
  store i64 0, ptr %ch_i, align 4
  br label %loop

loop:                                             ; preds = %idone, %entry
  %iv = load i64, ptr %ch_i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %3 = call { ptr, i64, i64 } @action_list_create(i64 %csafe)
  %ch_sa = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %3, ptr %ch_sa, align 8
  %ch_j = alloca i64, align 8
  store i64 0, ptr %ch_j, align 4
  br label %iloop

done:                                             ; preds = %loop
  %ch_rt = load { ptr, i64, i64 }, ptr %ch_ra, align 8
  ret { ptr, i64, i64 } %ch_rt

iloop:                                            ; preds = %ibody, %body
  %jv = load i64, ptr %ch_j, align 4
  %jc = icmp slt i64 %jv, %csafe
  %end = icmp sge i64 %iv, %len
  %4 = xor i1 %end, true
  %ic = and i1 %jc, %4
  br i1 %ic, label %ibody, label %idone

ibody:                                            ; preds = %iloop
  %cur_i = load i64, ptr %ch_i, align 4
  %ep = getelementptr %__action_str, ptr %data, i64 %cur_i
  %ev = load %__action_str, ptr %ep, align 8
  %cl = load { ptr, i64, i64 }, ptr %ch_sa, align 8
  %5 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %ev)
  store { ptr, i64, i64 } %5, ptr %ch_sa, align 8
  %ivi = add i64 %cur_i, 1
  store i64 %ivi, ptr %ch_i, align 4
  %jvi = add i64 %jv, 1
  store i64 %jvi, ptr %ch_j, align 4
  br label %iloop

idone:                                            ; preds = %iloop
  %sl = load { ptr, i64, i64 }, ptr %ch_sa, align 8
  %ch_sp = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %sl, ptr %ch_sp, align 8
  %sv = insertvalue %__action_str { i64 6, ptr undef }, ptr %ch_sp, 1
  %rl = load { ptr, i64, i64 }, ptr %ch_ra, align 8
  %6 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %rl, %__action_str %sv)
  store { ptr, i64, i64 } %6, ptr %ch_ra, align 8
  br label %loop
}

define { ptr, i64, i64 } @action_list_windows({ ptr, i64, i64 } %0, i64 %1) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %wz = icmp slt i64 %1, 1
  %wsafe = select i1 %wz, i64 1, i64 %1
  %tmp = sub i64 %len, %wsafe
  %nw1 = add i64 %tmp, 1
  %nz = icmp slt i64 %nw1, 0
  %nwin = select i1 %nz, i64 0, i64 %nw1
  %2 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %wn_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %2, ptr %wn_ra, align 8
  %wn_i = alloca i64, align 8
  store i64 0, ptr %wn_i, align 4
  br label %loop

loop:                                             ; preds = %idone, %entry
  %iv = load i64, ptr %wn_i, align 4
  %cond = icmp slt i64 %iv, %nwin
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %3 = call { ptr, i64, i64 } @action_list_create(i64 %wsafe)
  %wn_sa = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %3, ptr %wn_sa, align 8
  %wn_j = alloca i64, align 8
  store i64 0, ptr %wn_j, align 4
  br label %iloop

done:                                             ; preds = %loop
  %wn_rt = load { ptr, i64, i64 }, ptr %wn_ra, align 8
  ret { ptr, i64, i64 } %wn_rt

iloop:                                            ; preds = %ibody, %body
  %jv = load i64, ptr %wn_j, align 4
  %jc = icmp slt i64 %jv, %wsafe
  br i1 %jc, label %ibody, label %idone

ibody:                                            ; preds = %iloop
  %epi = add i64 %iv, %jv
  %ep = getelementptr %__action_str, ptr %data, i64 %epi
  %ev = load %__action_str, ptr %ep, align 8
  %cl = load { ptr, i64, i64 }, ptr %wn_sa, align 8
  %4 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %ev)
  store { ptr, i64, i64 } %4, ptr %wn_sa, align 8
  %jvi = add i64 %jv, 1
  store i64 %jvi, ptr %wn_j, align 4
  br label %iloop

idone:                                            ; preds = %iloop
  %sl = load { ptr, i64, i64 }, ptr %wn_sa, align 8
  %wn_sp = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %sl, ptr %wn_sp, align 8
  %fv = insertvalue %__action_str { i64 6, ptr undef }, ptr %wn_sp, 1
  %rl = load { ptr, i64, i64 }, ptr %wn_ra, align 8
  %5 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %rl, %__action_str %fv)
  store { ptr, i64, i64 } %5, ptr %wn_ra, align 8
  %ivi = add i64 %iv, 1
  store i64 %ivi, ptr %wn_i, align 4
  br label %loop
}

define i64 @action_list_index_of({ ptr, i64, i64 } %0, %__action_str %1) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %i = alloca i64, align 8
  store i64 0, ptr %i, align 4
  br label %loop

loop:                                             ; preds = %next, %entry
  %iv = load i64, ptr %i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %notfound

body:                                             ; preds = %loop
  %ep = getelementptr %__action_str, ptr %data, i64 %iv
  %ev = load %__action_str, ptr %ep, align 8
  %etag = extractvalue %__action_str %ev, 0
  %ttag = extractvalue %__action_str %1, 0
  %teq = icmp eq i64 %etag, %ttag
  %eptr = extractvalue %__action_str %ev, 1
  %tptr = extractvalue %__action_str %1, 1
  %2 = ptrtoint ptr %eptr to i64
  %3 = ptrtoint ptr %tptr to i64
  %scm = icmp eq i64 %2, %3
  %match = and i1 %teq, %scm
  br i1 %match, label %ret_match, label %next

notfound:                                         ; preds = %loop
  ret i64 -1

ret_match:                                        ; preds = %body
  ret i64 %iv

next:                                             ; preds = %body
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %i, align 4
  br label %loop
}

define double @action_abs_f(double %0) {
entry:
  %neg = fneg double %0
  %cmp = fcmp olt double %0, 0.000000e+00
  %r = select i1 %cmp, double %neg, double %0
  ret double %r
}

define { ptr, i64, i64 } @action_map_keys({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %mk_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %mk_ra, align 8
  %mk_i = alloca i64, align 8
  store i64 0, ptr %mk_i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %iv = load i64, ptr %mk_i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %off = mul i64 %iv, 4
  %ktp = getelementptr i64, ptr %data, i64 %off
  %kt = load i64, ptr %ktp, align 4
  %off1 = add i64 %off, 1
  %kpp = getelementptr i64, ptr %data, i64 %off1
  %kp_i64 = load i64, ptr %kpp, align 4
  %kp = inttoptr i64 %kp_i64 to ptr
  %ktag = insertvalue %__action_str undef, i64 %kt, 0
  %kdata = insertvalue %__action_str %ktag, ptr %kp, 1
  %cl = load { ptr, i64, i64 }, ptr %mk_ra, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %kdata)
  store { ptr, i64, i64 } %2, ptr %mk_ra, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %mk_i, align 4
  br label %loop

done:                                             ; preds = %loop
  %mk_rt = load { ptr, i64, i64 }, ptr %mk_ra, align 8
  ret { ptr, i64, i64 } %mk_rt
}

define { ptr, i64, i64 } @action_map_values({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %mv_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %mv_ra, align 8
  %mv_i = alloca i64, align 8
  store i64 0, ptr %mv_i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %iv = load i64, ptr %mv_i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %off = mul i64 %iv, 4
  %off2 = add i64 %off, 2
  %vtp = getelementptr i64, ptr %data, i64 %off2
  %vt = load i64, ptr %vtp, align 4
  %off3 = add i64 %off, 3
  %vpp = getelementptr i64, ptr %data, i64 %off3
  %vp_i64 = load i64, ptr %vpp, align 4
  %vp = inttoptr i64 %vp_i64 to ptr
  %vtag = insertvalue %__action_str undef, i64 %vt, 0
  %vdata = insertvalue %__action_str %vtag, ptr %vp, 1
  %cl = load { ptr, i64, i64 }, ptr %mv_ra, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %vdata)
  store { ptr, i64, i64 } %2, ptr %mv_ra, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %mv_i, align 4
  br label %loop

done:                                             ; preds = %loop
  %mv_rt = load { ptr, i64, i64 }, ptr %mv_ra, align 8
  ret { ptr, i64, i64 } %mv_rt
}

define { ptr, i64, i64 } @action_map_entries({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %me_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %me_ra, align 8
  %me_i = alloca i64, align 8
  store i64 0, ptr %me_i, align 4
  br label %loop

loop:                                             ; preds = %body, %entry
  %iv = load i64, ptr %me_i, align 4
  %cond = icmp slt i64 %iv, %len
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %off = mul i64 %iv, 4
  %ktp = getelementptr i64, ptr %data, i64 %off
  %kt = load i64, ptr %ktp, align 4
  %off1 = add i64 %off, 1
  %kpp = getelementptr i64, ptr %data, i64 %off1
  %kp_i64 = load i64, ptr %kpp, align 4
  %kp = inttoptr i64 %kp_i64 to ptr
  %off2 = add i64 %off, 2
  %vtp = getelementptr i64, ptr %data, i64 %off2
  %vt = load i64, ptr %vtp, align 4
  %off3 = add i64 %off, 3
  %vpp = getelementptr i64, ptr %data, i64 %off3
  %vp_i64 = load i64, ptr %vpp, align 4
  %vp = inttoptr i64 %vp_i64 to ptr
  %k1 = insertvalue %__action_str undef, i64 %kt, 0
  %k2 = insertvalue %__action_str %k1, ptr %kp, 1
  %v1 = insertvalue %__action_str undef, i64 %vt, 0
  %v2 = insertvalue %__action_str %v1, ptr %vp, 1
  %tup = call ptr @malloc(i64 32)
  store %__action_str %k2, ptr %tup, align 8
  %vslot = getelementptr %__action_str, ptr %tup, i64 1
  store %__action_str %v2, ptr %vslot, align 8
  %fdata = insertvalue %__action_str { i64 5, ptr undef }, ptr %tup, 1
  %cl = load { ptr, i64, i64 }, ptr %me_ra, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %cl, %__action_str %fdata)
  store { ptr, i64, i64 } %2, ptr %me_ra, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %me_i, align 4
  br label %loop

done:                                             ; preds = %loop
  %me_rt = load { ptr, i64, i64 }, ptr %me_ra, align 8
  ret { ptr, i64, i64 } %me_rt
}

define { ptr, i64, i64 } @action_set_union({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %alen = extractvalue { ptr, i64, i64 } %0, 1
  %blen = extractvalue { ptr, i64, i64 } %1, 1
  %cap = add i64 %alen, %blen
  %cap4 = add i64 %cap, 4
  %res = call { ptr, i64, i64 } @action_map_create(i64 %cap4)
  %su_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %res, ptr %su_ra, align 8
  %adata = extractvalue { ptr, i64, i64 } %0, 0
  %su_i = alloca i64, align 8
  store i64 0, ptr %su_i, align 4
  br label %loop1

loop1:                                            ; preds = %body1, %entry
  %iv = load i64, ptr %su_i, align 4
  %c1 = icmp slt i64 %iv, %alen
  br i1 %c1, label %body1, label %done1

body1:                                            ; preds = %loop1
  %off = mul i64 %iv, 4
  %tp = getelementptr i64, ptr %adata, i64 %off
  %tag = load i64, ptr %tp, align 4
  %off1 = add i64 %off, 1
  %pp = getelementptr i64, ptr %adata, i64 %off1
  %pi = load i64, ptr %pp, align 4
  %pv = inttoptr i64 %pi to ptr
  %k1 = insertvalue %__action_str undef, i64 %tag, 0
  %k2 = insertvalue %__action_str %k1, ptr %pv, 1
  %cl1 = load { ptr, i64, i64 }, ptr %su_ra, align 8
  %ins = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl1, %__action_str %k2, %__action_str zeroinitializer)
  store { ptr, i64, i64 } %ins, ptr %su_ra, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %su_i, align 4
  br label %loop1

done1:                                            ; preds = %loop1
  %bdata = extractvalue { ptr, i64, i64 } %1, 0
  %su_j = alloca i64, align 8
  store i64 0, ptr %su_j, align 4
  br label %loop2

loop2:                                            ; preds = %skip, %done1
  %jv = load i64, ptr %su_j, align 4
  %c2 = icmp slt i64 %jv, %blen
  br i1 %c2, label %body2, label %done2

body2:                                            ; preds = %loop2
  %boff = mul i64 %jv, 4
  %tp1 = getelementptr i64, ptr %bdata, i64 %boff
  %tag2 = load i64, ptr %tp1, align 4
  %off13 = add i64 %boff, 1
  %pp4 = getelementptr i64, ptr %bdata, i64 %off13
  %pi5 = load i64, ptr %pp4, align 4
  %pv6 = inttoptr i64 %pi5 to ptr
  %k17 = insertvalue %__action_str undef, i64 %tag2, 0
  %k28 = insertvalue %__action_str %k17, ptr %pv6, 1
  %cl2 = load { ptr, i64, i64 }, ptr %su_ra, align 8
  %cont = call i1 @action_map_contains({ ptr, i64, i64 } %cl2, %__action_str %k28)
  %nc = xor i1 %cont, true
  br i1 %nc, label %add, label %skip

done2:                                            ; preds = %loop2
  %su_rt = load { ptr, i64, i64 }, ptr %su_ra, align 8
  ret { ptr, i64, i64 } %su_rt

add:                                              ; preds = %body2
  %cl3 = load { ptr, i64, i64 }, ptr %su_ra, align 8
  %ins2 = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl3, %__action_str %k28, %__action_str zeroinitializer)
  store { ptr, i64, i64 } %ins2, ptr %su_ra, align 8
  br label %skip

skip:                                             ; preds = %add, %body2
  %inc2 = add i64 %jv, 1
  store i64 %inc2, ptr %su_j, align 4
  br label %loop2
}

define { ptr, i64, i64 } @action_set_intersection({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %alen = extractvalue { ptr, i64, i64 } %0, 1
  %cap4 = add i64 %alen, 4
  %res = call { ptr, i64, i64 } @action_map_create(i64 %cap4)
  %si_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %res, ptr %si_ra, align 8
  %adata = extractvalue { ptr, i64, i64 } %0, 0
  %si_i = alloca i64, align 8
  store i64 0, ptr %si_i, align 4
  br label %loop

loop:                                             ; preds = %skip, %entry
  %iv = load i64, ptr %si_i, align 4
  %cond = icmp slt i64 %iv, %alen
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %off = mul i64 %iv, 4
  %tp = getelementptr i64, ptr %adata, i64 %off
  %tag = load i64, ptr %tp, align 4
  %off1 = add i64 %off, 1
  %pp = getelementptr i64, ptr %adata, i64 %off1
  %pi = load i64, ptr %pp, align 4
  %pv = inttoptr i64 %pi to ptr
  %k1 = insertvalue %__action_str undef, i64 %tag, 0
  %k2 = insertvalue %__action_str %k1, ptr %pv, 1
  %cont = call i1 @action_map_contains({ ptr, i64, i64 } %1, %__action_str %k2)
  br i1 %cont, label %add, label %skip

done:                                             ; preds = %loop
  %si_rt = load { ptr, i64, i64 }, ptr %si_ra, align 8
  ret { ptr, i64, i64 } %si_rt

add:                                              ; preds = %body
  %cl2 = load { ptr, i64, i64 }, ptr %si_ra, align 8
  %ins = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl2, %__action_str %k2, %__action_str zeroinitializer)
  store { ptr, i64, i64 } %ins, ptr %si_ra, align 8
  br label %skip

skip:                                             ; preds = %add, %body
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %si_i, align 4
  br label %loop
}

define { ptr, i64, i64 } @action_set_difference({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %alen = extractvalue { ptr, i64, i64 } %0, 1
  %cap4 = add i64 %alen, 4
  %res = call { ptr, i64, i64 } @action_map_create(i64 %cap4)
  %sd_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %res, ptr %sd_ra, align 8
  %adata = extractvalue { ptr, i64, i64 } %0, 0
  %sd_i = alloca i64, align 8
  store i64 0, ptr %sd_i, align 4
  br label %loop

loop:                                             ; preds = %skip, %entry
  %iv = load i64, ptr %sd_i, align 4
  %cond = icmp slt i64 %iv, %alen
  br i1 %cond, label %body, label %done

body:                                             ; preds = %loop
  %off = mul i64 %iv, 4
  %tp = getelementptr i64, ptr %adata, i64 %off
  %tag = load i64, ptr %tp, align 4
  %off1 = add i64 %off, 1
  %pp = getelementptr i64, ptr %adata, i64 %off1
  %pi = load i64, ptr %pp, align 4
  %pv = inttoptr i64 %pi to ptr
  %k1 = insertvalue %__action_str undef, i64 %tag, 0
  %k2 = insertvalue %__action_str %k1, ptr %pv, 1
  %cont = call i1 @action_map_contains({ ptr, i64, i64 } %1, %__action_str %k2)
  %nc = xor i1 %cont, true
  br i1 %nc, label %add, label %skip

done:                                             ; preds = %loop
  %sd_rt = load { ptr, i64, i64 }, ptr %sd_ra, align 8
  ret { ptr, i64, i64 } %sd_rt

add:                                              ; preds = %body
  %cl2 = load { ptr, i64, i64 }, ptr %sd_ra, align 8
  %ins = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl2, %__action_str %k2, %__action_str zeroinitializer)
  store { ptr, i64, i64 } %ins, ptr %sd_ra, align 8
  br label %skip

skip:                                             ; preds = %add, %body
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %sd_i, align 4
  br label %loop
}

define i1 @action_set_is_subset({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %ad = extractvalue { ptr, i64, i64 } %0, 0
  %al = extractvalue { ptr, i64, i64 } %0, 1
  %bd = extractvalue { ptr, i64, i64 } %1, 0
  %bl = extractvalue { ptr, i64, i64 } %1, 1
  %oi = alloca i64, align 8
  store i64 0, ptr %oi, align 4
  br label %oloop

oloop:                                            ; preds = %oinc, %entry
  %oiv = load i64, ptr %oi, align 4
  %ocond = icmp slt i64 %oiv, %al
  br i1 %ocond, label %obody, label %rtrue

obody:                                            ; preds = %oloop
  %a_off = mul i64 %oiv, 4
  %a_tp = getelementptr i64, ptr %ad, i64 %a_off
  %a_tag = load i64, ptr %a_tp, align 4
  %a_off1 = add i64 %a_off, 1
  %a_pp = getelementptr i64, ptr %ad, i64 %a_off1
  %a_pi = load i64, ptr %a_pp, align 4
  %a_is_null = icmp eq i64 %a_pi, 0
  %ij = alloca i64, align 8
  store i64 0, ptr %ij, align 4
  br label %iloop

ofound:                                           ; preds = %istr_found, %ifound_bb
  br label %oinc

oinc:                                             ; preds = %ofound
  %noi = add i64 %oiv, 1
  store i64 %noi, ptr %oi, align 4
  br label %oloop

rtrue:                                            ; preds = %oloop
  ret i1 true

rfalse:                                           ; preds = %inotfound
  ret i1 false

iloop:                                            ; preds = %inext, %obody
  %ijv = load i64, ptr %ij, align 4
  %icond = icmp slt i64 %ijv, %bl
  br i1 %icond, label %ibody, label %inotfound

ibody:                                            ; preds = %iloop
  %b_off = mul i64 %ijv, 4
  %b_tp = getelementptr i64, ptr %bd, i64 %b_off
  %b_tag = load i64, ptr %b_tp, align 4
  %b_off1 = add i64 %b_off, 1
  %b_pp = getelementptr i64, ptr %bd, i64 %b_off1
  %b_pi = load i64, ptr %b_pp, align 4
  %tag_eq = icmp eq i64 %a_tag, %b_tag
  br i1 %tag_eq, label %icontent, label %inext

inext:                                            ; preds = %istr_eq, %istr_bb, %ibody
  %nij = add i64 %ijv, 1
  store i64 %nij, ptr %ij, align 4
  br label %iloop

inotfound:                                        ; preds = %iloop
  br label %rfalse

icontent:                                         ; preds = %ibody
  %b_is_null = icmp eq i64 %b_pi, 0
  %both_null = and i1 %a_is_null, %b_is_null
  br i1 %both_null, label %ifound_bb, label %istr_bb

ifound_bb:                                        ; preds = %icontent
  br label %ofound

istr_bb:                                          ; preds = %icontent
  %a_nn = xor i1 %a_is_null, true
  %b_nn = xor i1 %b_is_null, true
  %both_nn = and i1 %a_nn, %b_nn
  br i1 %both_nn, label %istr_eq, label %inext

istr_eq:                                          ; preds = %istr_bb
  %af1 = insertvalue %__action_str undef, i64 %a_tag, 0
  %a_ptr = inttoptr i64 %a_pi to ptr
  %af2 = insertvalue %__action_str %af1, ptr %a_ptr, 1
  %bf1 = insertvalue %__action_str undef, i64 %b_tag, 0
  %b_ptr = inttoptr i64 %b_pi to ptr
  %bf2 = insertvalue %__action_str %bf1, ptr %b_ptr, 1
  %sseq = call i1 @action_string_eq(%__action_str %af2, %__action_str %bf2)
  br i1 %sseq, label %istr_found, label %inext

istr_found:                                       ; preds = %istr_eq
  br label %ofound
}

define { ptr, i64, i64 } @action_rand_shuffle({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %rs_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %rs_ra, align 8
  %rs_ci = alloca i64, align 8
  store i64 0, ptr %rs_ci, align 4
  br label %cloop

cloop:                                            ; preds = %cbody, %entry
  %civ = load i64, ptr %rs_ci, align 4
  %ccond = icmp slt i64 %civ, %len
  br i1 %ccond, label %cbody, label %cdone

cbody:                                            ; preds = %cloop
  %cep = getelementptr %__action_str, ptr %data, i64 %civ
  %cev = load %__action_str, ptr %cep, align 8
  %ccl = load { ptr, i64, i64 }, ptr %rs_ra, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %ccl, %__action_str %cev)
  store { ptr, i64, i64 } %2, ptr %rs_ra, align 8
  %cinc = add i64 %civ, 1
  store i64 %cinc, ptr %rs_ci, align 4
  br label %cloop

cdone:                                            ; preds = %cloop
  %rs_i = alloca i64, align 8
  %len1 = sub i64 %len, 1
  store i64 %len1, ptr %rs_i, align 4
  br label %floop

floop:                                            ; preds = %fbody, %cdone
  %iv = load i64, ptr %rs_i, align 4
  %fcond = icmp sgt i64 %iv, 0
  br i1 %fcond, label %fbody, label %fdone

fbody:                                            ; preds = %floop
  %3 = call i64 @action_rand_int(i64 0, i64 %iv)
  %cur_list = load { ptr, i64, i64 }, ptr %rs_ra, align 8
  %cur_data = extractvalue { ptr, i64, i64 } %cur_list, 0
  %epi = getelementptr %__action_str, ptr %cur_data, i64 %iv
  %epj = getelementptr %__action_str, ptr %cur_data, i64 %3
  %ei = load %__action_str, ptr %epi, align 8
  %ej = load %__action_str, ptr %epj, align 8
  store %__action_str %ej, ptr %epi, align 8
  store %__action_str %ei, ptr %epj, align 8
  %dec = sub i64 %iv, 1
  store i64 %dec, ptr %rs_i, align 4
  br label %floop

fdone:                                            ; preds = %floop
  %rs_rt = load { ptr, i64, i64 }, ptr %rs_ra, align 8
  ret { ptr, i64, i64 } %rs_rt
}

define { ptr, i64, i64 } @action_list_sorted({ ptr, i64, i64 } %0) {
entry:
  %data = extractvalue { ptr, i64, i64 } %0, 0
  %len = extractvalue { ptr, i64, i64 } %0, 1
  %1 = call { ptr, i64, i64 } @action_list_create(i64 4)
  %srt_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %srt_ra, align 8
  %srt_ci = alloca i64, align 8
  store i64 0, ptr %srt_ci, align 4
  br label %cloop

cloop:                                            ; preds = %cbody, %entry
  %civ = load i64, ptr %srt_ci, align 4
  %ccond = icmp slt i64 %civ, %len
  br i1 %ccond, label %cbody, label %cdone

cbody:                                            ; preds = %cloop
  %cep = getelementptr %__action_str, ptr %data, i64 %civ
  %cev = load %__action_str, ptr %cep, align 8
  %ccl = load { ptr, i64, i64 }, ptr %srt_ra, align 8
  %2 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %ccl, %__action_str %cev)
  store { ptr, i64, i64 } %2, ptr %srt_ra, align 8
  %cinc = add i64 %civ, 1
  store i64 %cinc, ptr %srt_ci, align 4
  br label %cloop

cdone:                                            ; preds = %cloop
  %srt_i = alloca i64, align 8
  store i64 0, ptr %srt_i, align 4
  br label %oloop

oloop:                                            ; preds = %idone, %cdone
  %iv = load i64, ptr %srt_i, align 4
  %ocond = icmp slt i64 %iv, %len
  br i1 %ocond, label %obody, label %odone

obody:                                            ; preds = %oloop
  %srt_j = alloca i64, align 8
  store i64 0, ptr %srt_j, align 4
  %len1 = sub i64 %len, 1
  br label %iloop

odone:                                            ; preds = %oloop
  %srt_rt = load { ptr, i64, i64 }, ptr %srt_ra, align 8
  ret { ptr, i64, i64 } %srt_rt

iloop:                                            ; preds = %noswap, %obody
  %jv = load i64, ptr %srt_j, align 4
  %jc = icmp slt i64 %jv, %len1
  br i1 %jc, label %ibody, label %idone

ibody:                                            ; preds = %iloop
  %cur = load { ptr, i64, i64 }, ptr %srt_ra, align 8
  %curd = extractvalue { ptr, i64, i64 } %cur, 0
  %epa = getelementptr %__action_str, ptr %curd, i64 %jv
  %jp1 = add i64 %jv, 1
  %epb = getelementptr %__action_str, ptr %curd, i64 %jp1
  %ea = load %__action_str, ptr %epa, align 8
  %eb = load %__action_str, ptr %epb, align 8
  %eat = extractvalue %__action_str %ea, 0
  %ebt = extractvalue %__action_str %eb, 0
  %isint = icmp eq i64 %eat, 0
  %eap = extractvalue %__action_str %ea, 1
  %ebp = extractvalue %__action_str %eb, 1
  %eai = ptrtoint ptr %eap to i64
  %ebi = ptrtoint ptr %ebp to i64
  %swap = icmp sgt i64 %eai, %ebi
  br i1 %swap, label %swap1, label %noswap

idone:                                            ; preds = %iloop
  %iinc = add i64 %iv, 1
  store i64 %iinc, ptr %srt_i, align 4
  br label %oloop

swap1:                                            ; preds = %ibody
  store %__action_str %eb, ptr %epa, align 8
  store %__action_str %ea, ptr %epb, align 8
  br label %noswap

noswap:                                           ; preds = %swap1, %ibody
  %jinc = add i64 %jv, 1
  store i64 %jinc, ptr %srt_j, align 4
  br label %iloop
}

define { ptr, i64, i64 } @action_map_union({ ptr, i64, i64 } %0, { ptr, i64, i64 } %1) {
entry:
  %alen = extractvalue { ptr, i64, i64 } %0, 1
  %blen = extractvalue { ptr, i64, i64 } %1, 1
  %cap = add i64 %alen, %blen
  %cap4 = add i64 %cap, 4
  %res = call { ptr, i64, i64 } @action_map_create(i64 %cap4)
  %mu_ra = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %res, ptr %mu_ra, align 8
  %adata = extractvalue { ptr, i64, i64 } %0, 0
  %mu_i = alloca i64, align 8
  store i64 0, ptr %mu_i, align 4
  br label %loop1

loop1:                                            ; preds = %body1, %entry
  %iv = load i64, ptr %mu_i, align 4
  %c1 = icmp slt i64 %iv, %alen
  br i1 %c1, label %body1, label %done1

body1:                                            ; preds = %loop1
  %off = mul i64 %iv, 4
  %ktp = getelementptr i64, ptr %adata, i64 %off
  %kt = load i64, ptr %ktp, align 4
  %off1 = add i64 %off, 1
  %kpp = getelementptr i64, ptr %adata, i64 %off1
  %kpi = load i64, ptr %kpp, align 4
  %kp = inttoptr i64 %kpi to ptr
  %off2 = add i64 %off, 2
  %vtp = getelementptr i64, ptr %adata, i64 %off2
  %vt = load i64, ptr %vtp, align 4
  %off3 = add i64 %off, 3
  %vpp = getelementptr i64, ptr %adata, i64 %off3
  %vpi = load i64, ptr %vpp, align 4
  %vp = inttoptr i64 %vpi to ptr
  %k1 = insertvalue %__action_str undef, i64 %kt, 0
  %kf = insertvalue %__action_str %k1, ptr %kp, 1
  %v1 = insertvalue %__action_str undef, i64 %vt, 0
  %vf = insertvalue %__action_str %v1, ptr %vp, 1
  %cl1 = load { ptr, i64, i64 }, ptr %mu_ra, align 8
  %ins = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl1, %__action_str %kf, %__action_str %vf)
  store { ptr, i64, i64 } %ins, ptr %mu_ra, align 8
  %inc = add i64 %iv, 1
  store i64 %inc, ptr %mu_i, align 4
  br label %loop1

done1:                                            ; preds = %loop1
  %bdata = extractvalue { ptr, i64, i64 } %1, 0
  %mu_j = alloca i64, align 8
  store i64 0, ptr %mu_j, align 4
  br label %loop2

loop2:                                            ; preds = %body2, %done1
  %jv = load i64, ptr %mu_j, align 4
  %c2 = icmp slt i64 %jv, %blen
  br i1 %c2, label %body2, label %done2

body2:                                            ; preds = %loop2
  %boff = mul i64 %jv, 4
  %ktp1 = getelementptr i64, ptr %bdata, i64 %boff
  %kt2 = load i64, ptr %ktp1, align 4
  %off13 = add i64 %boff, 1
  %kpp4 = getelementptr i64, ptr %bdata, i64 %off13
  %kpi5 = load i64, ptr %kpp4, align 4
  %kp6 = inttoptr i64 %kpi5 to ptr
  %off27 = add i64 %boff, 2
  %vtp8 = getelementptr i64, ptr %bdata, i64 %off27
  %vt9 = load i64, ptr %vtp8, align 4
  %off310 = add i64 %boff, 3
  %vpp11 = getelementptr i64, ptr %bdata, i64 %off310
  %vpi12 = load i64, ptr %vpp11, align 4
  %vp13 = inttoptr i64 %vpi12 to ptr
  %k114 = insertvalue %__action_str undef, i64 %kt2, 0
  %kf15 = insertvalue %__action_str %k114, ptr %kp6, 1
  %v116 = insertvalue %__action_str undef, i64 %vt9, 0
  %vf17 = insertvalue %__action_str %v116, ptr %vp13, 1
  %cl2 = load { ptr, i64, i64 }, ptr %mu_ra, align 8
  %ins2 = call { ptr, i64, i64 } @action_map_insert({ ptr, i64, i64 } %cl2, %__action_str %kf15, %__action_str %vf17)
  store { ptr, i64, i64 } %ins2, ptr %mu_ra, align 8
  %inc2 = add i64 %jv, 1
  store i64 %inc2, ptr %mu_j, align 4
  br label %loop2

done2:                                            ; preds = %loop2
  %mu_rt = load { ptr, i64, i64 }, ptr %mu_ra, align 8
  ret { ptr, i64, i64 } %mu_rt
}

define double @action_pow(double %0, double %1) {
entry:
  %r = call double @pow(double %0, double %1)
  ret double %r
}

define void @action_rc_inc(ptr %0) {
entry:
  %is_null = icmp eq ptr %0, null
  br i1 %is_null, label %done, label %do_inc

do_inc:                                           ; preds = %entry
  %rc_i64 = ptrtoint ptr %0 to i64
  %minus8 = sub i64 %rc_i64, 8
  %rc_i64p = inttoptr i64 %minus8 to ptr
  %rc = load i64, ptr %rc_i64p, align 4
  %new_rc = add i64 %rc, 1
  store i64 %new_rc, ptr %rc_i64p, align 4
  br label %done

done:                                             ; preds = %do_inc, %entry
  ret void
}

define void @action_rc_dec(ptr %0) {
entry:
  %is_null = icmp eq ptr %0, null
  br i1 %is_null, label %done, label %null_check

null_check:                                       ; preds = %entry
  %rc_i64 = ptrtoint ptr %0 to i64
  %minus8 = sub i64 %rc_i64, 8
  %rc_i64p = inttoptr i64 %minus8 to ptr
  %rc = load i64, ptr %rc_i64p, align 4
  %new_rc = sub i64 %rc, 1
  store i64 %new_rc, ptr %rc_i64p, align 4
  %is_zero = icmp eq i64 %new_rc, 0
  br i1 %is_zero, label %do_free, label %done

do_free:                                          ; preds = %null_check
  %free_ptr = inttoptr i64 %minus8 to ptr
  call void @free(ptr %free_ptr)
  br label %done

done:                                             ; preds = %do_free, %null_check, %entry
  ret void
}

define void @action_rc_dec_list_node(ptr %0, i64 %1) {
entry:
  %is_null = icmp eq ptr %0, null
  br i1 %is_null, label %null_done, label %do_dec

null_done:                                        ; preds = %entry
  ret void

do_dec:                                           ; preds = %entry
  %pi64 = ptrtoint ptr %0 to i64
  %rc_addr = sub i64 %pi64, 8
  %rc_p = inttoptr i64 %rc_addr to ptr
  %rc = load i64, ptr %rc_p, align 4
  %new_rc = sub i64 %rc, 1
  store i64 %new_rc, ptr %rc_p, align 4
  br label %check_zero

check_zero:                                       ; preds = %do_dec
  %is_zero = icmp eq i64 %new_rc, 0
  br i1 %is_zero, label %leaf_cleanup, label %done

done:                                             ; preds = %check_zero
  ret void

leaf_cleanup:                                     ; preds = %check_zero
  %is_leaf = icmp eq i64 %1, 0
  br i1 %is_leaf, label %int_cleanup, label %int_cleanup

int_cleanup:                                      ; preds = %leaf_cleanup, %leaf_cleanup
  %count = load i64, ptr %0, align 4
  %count_zero = icmp eq i64 %count, 0
  br i1 %count_zero, label %free_node, label %iter_body

free_node:                                        ; preds = %iter_body, %int_cleanup
  %free_p = inttoptr i64 %rc_addr to ptr
  call void @free(ptr %free_p)
  ret void

iter_body:                                        ; preds = %call_skip, %int_cleanup
  %phi_i = phi i64 [ 0, %int_cleanup ], [ %next_i, %call_skip ]
  %done_cond = icmp sge i64 %phi_i, %count
  br i1 %done_cond, label %free_node, label %iter_next

iter_next:                                        ; preds = %iter_body
  %i16 = mul i64 %phi_i, 16
  %off = add i64 16, %i16
  %ep = getelementptr i8, ptr %0, i64 %off
  %ptr_val = load ptr, ptr %ep, align 8
  %ptr_nonnull = icmp ne ptr %ptr_val, null
  br i1 %ptr_nonnull, label %call_do, label %call_skip

call_skip:                                        ; preds = %call_int, %call_leaf, %iter_next
  %next_i = add i64 %phi_i, 1
  br label %iter_body

call_do:                                          ; preds = %iter_next
  br i1 %is_leaf, label %call_leaf, label %call_int

call_leaf:                                        ; preds = %call_do
  call void @action_rc_dec(ptr %ptr_val)
  br label %call_skip

call_int:                                         ; preds = %call_do
  %child_h = sub i64 %1, 1
  call void @action_rc_dec_list_node(ptr %ptr_val, i64 %child_h)
  br label %call_skip
}

define i64 @main() {
entry:
  %when_result = alloca i64, align 8
  br label %tail_entry

tail_entry:                                       ; preds = %entry
  %0 = call { ptr, i64, i64 } @action_list_create(i64 5)
  %collect_result = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %0, ptr %collect_result, align 8
  %collect_pos = alloca i64, align 8
  store i64 0, ptr %collect_pos, align 4
  %for_idx = alloca i64, align 8
  store i64 1, ptr %for_idx, align 4
  br label %for_header

for_header:                                       ; preds = %for_next, %tail_entry
  %i_val = load i64, ptr %for_idx, align 4
  %for_cond = icmp slt i64 %i_val, 6
  br i1 %for_cond, label %for_body, label %for_exit

for_body:                                         ; preds = %for_header
  %x = load i64, ptr %for_idx, align 4
  %x1 = load i64, ptr %for_idx, align 4
  %mul = mul i64 %x, %x1
  %list_load = load { ptr, i64, i64 }, ptr %collect_result, align 8
  %wrap0 = insertvalue %__action_str undef, i64 %mul, 0
  %wrap1 = insertvalue %__action_str %wrap0, ptr null, 1
  %list_data = extractvalue { ptr, i64, i64 } %list_load, 0
  %pos_val = load i64, ptr %collect_pos, align 4
  %collect_elem = getelementptr %__action_str, ptr %list_data, i64 %pos_val
  store %__action_str %wrap1, ptr %collect_elem, align 8
  %pos_next = add i64 %pos_val, 1
  store i64 %pos_next, ptr %collect_pos, align 4
  br label %for_next

for_next:                                         ; preds = %for_body
  %i_next = load i64, ptr %for_idx, align 4
  %i_inc = add i64 %i_next, 1
  store i64 %i_inc, ptr %for_idx, align 4
  br label %for_header

for_exit:                                         ; preds = %for_header
  %squares = alloca { ptr, i64, i64 }, align 8
  %list_load2 = load { ptr, i64, i64 }, ptr %collect_result, align 8
  store { ptr, i64, i64 } %list_load2, ptr %squares, align 8
  %list_load3 = load { ptr, i64, i64 }, ptr %collect_result, align 8
  %data = extractvalue { ptr, i64, i64 } %list_load3, 0
  call void @action_rc_inc(ptr %data)
  %1 = call { ptr, i64, i64 } @action_list_create(i64 10)
  %collect_result4 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %1, ptr %collect_result4, align 8
  %collect_pos5 = alloca i64, align 8
  store i64 0, ptr %collect_pos5, align 4
  %for_idx6 = alloca i64, align 8
  store i64 1, ptr %for_idx6, align 4
  br label %for_header7

for_header7:                                      ; preds = %for_next9, %for_exit
  %i_val11 = load i64, ptr %for_idx6, align 4
  %for_cond12 = icmp slt i64 %i_val11, 11
  br i1 %for_cond12, label %for_body8, label %for_exit10

for_body8:                                        ; preds = %for_header7
  %x13 = load i64, ptr %for_idx6, align 4
  %mod = srem i64 %x13, 2
  %eq = icmp eq i64 %mod, 0
  br i1 %eq, label %when_then, label %when_else

for_next9:                                        ; preds = %when_merge, %when_else
  %i_next22 = load i64, ptr %for_idx6, align 4
  %i_inc23 = add i64 %i_next22, 1
  store i64 %i_inc23, ptr %for_idx6, align 4
  br label %for_header7

for_exit10:                                       ; preds = %for_header7
  %evens = alloca { ptr, i64, i64 }, align 8
  %list_load24 = load { ptr, i64, i64 }, ptr %collect_result4, align 8
  store { ptr, i64, i64 } %list_load24, ptr %evens, align 8
  %list_load25 = load { ptr, i64, i64 }, ptr %collect_result4, align 8
  %data26 = extractvalue { ptr, i64, i64 } %list_load25, 0
  call void @action_rc_inc(ptr %data26)
  %2 = call { ptr, i64, i64 } @action_list_create(i64 3)
  %list_tmp = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %2, ptr %list_tmp, align 8
  %list_load27 = load { ptr, i64, i64 }, ptr %list_tmp, align 8
  %3 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %list_load27, %__action_str { i64 1, ptr null })
  store { ptr, i64, i64 } %3, ptr %list_tmp, align 8
  %list_load28 = load { ptr, i64, i64 }, ptr %list_tmp, align 8
  %4 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %list_load28, %__action_str { i64 2, ptr null })
  store { ptr, i64, i64 } %4, ptr %list_tmp, align 8
  %list_load29 = load { ptr, i64, i64 }, ptr %list_tmp, align 8
  %5 = call { ptr, i64, i64 } @action_list_push({ ptr, i64, i64 } %list_load29, %__action_str { i64 3, ptr null })
  store { ptr, i64, i64 } %5, ptr %list_tmp, align 8
  %list_load30 = load { ptr, i64, i64 }, ptr %list_tmp, align 8
  %list_len = extractvalue { ptr, i64, i64 } %list_load30, 1
  %est_len = sub i64 %list_len, 0
  %6 = call { ptr, i64, i64 } @action_list_create(i64 %est_len)
  %collect_result31 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %6, ptr %collect_result31, align 8
  %collect_pos32 = alloca i64, align 8
  store i64 0, ptr %collect_pos32, align 4
  %for_idx33 = alloca i64, align 8
  store i64 0, ptr %for_idx33, align 4
  %for_val = alloca i64, align 8
  br label %for_header34

when_then:                                        ; preds = %for_body8
  %x14 = load i64, ptr %for_idx6, align 4
  store i64 %x14, ptr %when_result, align 4
  br label %when_merge

when_else:                                        ; preds = %for_body8
  br label %for_next9

when_merge:                                       ; preds = %when_then
  %when_ld = load i64, ptr %when_result, align 4
  %list_load15 = load { ptr, i64, i64 }, ptr %collect_result4, align 8
  %wrap016 = insertvalue %__action_str undef, i64 %when_ld, 0
  %wrap117 = insertvalue %__action_str %wrap016, ptr null, 1
  %list_data18 = extractvalue { ptr, i64, i64 } %list_load15, 0
  %pos_val19 = load i64, ptr %collect_pos5, align 4
  %collect_elem20 = getelementptr %__action_str, ptr %list_data18, i64 %pos_val19
  store %__action_str %wrap117, ptr %collect_elem20, align 8
  %pos_next21 = add i64 %pos_val19, 1
  store i64 %pos_next21, ptr %collect_pos5, align 4
  br label %for_next9

for_header34:                                     ; preds = %for_next36, %for_exit10
  %i_val38 = load i64, ptr %for_idx33, align 4
  %for_cond39 = icmp slt i64 %i_val38, %list_len
  br i1 %for_cond39, label %for_body35, label %for_exit37

for_body35:                                       ; preds = %for_header34
  %list_load40 = load { ptr, i64, i64 }, ptr %list_tmp, align 8
  %list_data41 = extractvalue { ptr, i64, i64 } %list_load40, 0
  %fat_elem = getelementptr %__action_str, ptr %list_data41, i64 %i_val38
  %fat_val = load %__action_str, ptr %fat_elem, align 8
  %elem_tag = extractvalue %__action_str %fat_val, 0
  store i64 %elem_tag, ptr %for_val, align 4
  %it = load i64, ptr %for_val, align 4
  %it42 = load i64, ptr %for_val, align 4
  %mul43 = mul i64 %it, %it42
  %it44 = load i64, ptr %for_val, align 4
  %mul45 = mul i64 %mul43, %it44
  %list_load46 = load { ptr, i64, i64 }, ptr %collect_result31, align 8
  %wrap047 = insertvalue %__action_str undef, i64 %mul45, 0
  %wrap148 = insertvalue %__action_str %wrap047, ptr null, 1
  %list_data49 = extractvalue { ptr, i64, i64 } %list_load46, 0
  %pos_val50 = load i64, ptr %collect_pos32, align 4
  %collect_elem51 = getelementptr %__action_str, ptr %list_data49, i64 %pos_val50
  store %__action_str %wrap148, ptr %collect_elem51, align 8
  %pos_next52 = add i64 %pos_val50, 1
  store i64 %pos_next52, ptr %collect_pos32, align 4
  br label %for_next36

for_next36:                                       ; preds = %for_body35
  %i_next53 = load i64, ptr %for_idx33, align 4
  %i_inc54 = add i64 %i_next53, 1
  store i64 %i_inc54, ptr %for_idx33, align 4
  br label %for_header34

for_exit37:                                       ; preds = %for_header34
  %cubes = alloca { ptr, i64, i64 }, align 8
  %list_load55 = load { ptr, i64, i64 }, ptr %collect_result31, align 8
  store { ptr, i64, i64 } %list_load55, ptr %cubes, align 8
  %list_load56 = load { ptr, i64, i64 }, ptr %collect_result31, align 8
  %data57 = extractvalue { ptr, i64, i64 } %list_load56, 0
  call void @action_rc_inc(ptr %data57)
  %squares58 = load { ptr, i64, i64 }, ptr %squares, align 8
  %list_tmp59 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %squares58, ptr %list_tmp59, align 8
  %list_load60 = load { ptr, i64, i64 }, ptr %list_tmp59, align 8
  %7 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load60, i64 0)
  %list_elem = alloca %__action_str, align 8
  store %__action_str %7, ptr %list_elem, align 8
  %str_load = load %__action_str, ptr %list_elem, align 8
  call void @action_print_string(%__action_str %str_load)
  %squares61 = load { ptr, i64, i64 }, ptr %squares, align 8
  %list_tmp62 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %squares61, ptr %list_tmp62, align 8
  %list_load63 = load { ptr, i64, i64 }, ptr %list_tmp62, align 8
  %8 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load63, i64 4)
  %list_elem64 = alloca %__action_str, align 8
  store %__action_str %8, ptr %list_elem64, align 8
  %str_load65 = load %__action_str, ptr %list_elem64, align 8
  call void @action_print_string(%__action_str %str_load65)
  %evens66 = load { ptr, i64, i64 }, ptr %evens, align 8
  %list_tmp67 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %evens66, ptr %list_tmp67, align 8
  %list_load68 = load { ptr, i64, i64 }, ptr %list_tmp67, align 8
  %9 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load68, i64 0)
  %list_elem69 = alloca %__action_str, align 8
  store %__action_str %9, ptr %list_elem69, align 8
  %str_load70 = load %__action_str, ptr %list_elem69, align 8
  call void @action_print_string(%__action_str %str_load70)
  %evens71 = load { ptr, i64, i64 }, ptr %evens, align 8
  %list_tmp72 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %evens71, ptr %list_tmp72, align 8
  %list_load73 = load { ptr, i64, i64 }, ptr %list_tmp72, align 8
  %10 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load73, i64 4)
  %list_elem74 = alloca %__action_str, align 8
  store %__action_str %10, ptr %list_elem74, align 8
  %str_load75 = load %__action_str, ptr %list_elem74, align 8
  call void @action_print_string(%__action_str %str_load75)
  %cubes76 = load { ptr, i64, i64 }, ptr %cubes, align 8
  %list_tmp77 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %cubes76, ptr %list_tmp77, align 8
  %list_load78 = load { ptr, i64, i64 }, ptr %list_tmp77, align 8
  %11 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load78, i64 0)
  %list_elem79 = alloca %__action_str, align 8
  store %__action_str %11, ptr %list_elem79, align 8
  %str_load80 = load %__action_str, ptr %list_elem79, align 8
  call void @action_print_string(%__action_str %str_load80)
  %cubes81 = load { ptr, i64, i64 }, ptr %cubes, align 8
  %list_tmp82 = alloca { ptr, i64, i64 }, align 8
  store { ptr, i64, i64 } %cubes81, ptr %list_tmp82, align 8
  %list_load83 = load { ptr, i64, i64 }, ptr %list_tmp82, align 8
  %12 = call %__action_str @action_list_get({ ptr, i64, i64 } %list_load83, i64 2)
  %list_elem84 = alloca %__action_str, align 8
  store %__action_str %12, ptr %list_elem84, align 8
  %str_load85 = load %__action_str, ptr %list_elem84, align 8
  call void @action_print_string(%__action_str %str_load85)
  %list_load86 = load { ptr, i64, i64 }, ptr %cubes, align 8
  %data87 = extractvalue { ptr, i64, i64 } %list_load86, 0
  %height = extractvalue { ptr, i64, i64 } %list_load86, 2
  call void @action_rc_dec_list_node(ptr %data87, i64 %height)
  %list_load88 = load { ptr, i64, i64 }, ptr %evens, align 8
  %data89 = extractvalue { ptr, i64, i64 } %list_load88, 0
  %height90 = extractvalue { ptr, i64, i64 } %list_load88, 2
  call void @action_rc_dec_list_node(ptr %data89, i64 %height90)
  %list_load91 = load { ptr, i64, i64 }, ptr %squares, align 8
  %data92 = extractvalue { ptr, i64, i64 } %list_load91, 0
  %height93 = extractvalue { ptr, i64, i64 } %list_load91, 2
  call void @action_rc_dec_list_node(ptr %data92, i64 %height93)
  %13 = call i32 @fflush(ptr null)
  ret i64 0
}

attributes #0 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
