use nu_test_support::nu;

#[test]
fn capture_errors_works() {
    let actual = nu!("do -c {$env.use}");

    eprintln!("actual.err: {:?}", actual.err);

    assert!(actual.err.contains("column_not_found"));
}

#[test]
fn capture_errors_works_for_external() {
    let actual = nu!("do -c {nu --testbin fail}");
    assert!(actual.err.contains("External command failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_pipeline() {
    let actual = nu!("do -c {nu --testbin fail} | echo `text`");
    assert!(actual.err.contains("External command failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -c {nu --testbin fail}; echo `text`"#);
    assert!(actual.err.contains("External command failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn do_with_semicolon_break_on_failed_external() {
    let actual = nu!(r#"do { nu --not_exist_flag }; `text`"#);

    assert_eq!(actual.out, "");
}

#[test]
fn ignore_shell_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -s { open asdfasdf.txt }; "text""#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "text");
}

#[test]
fn ignore_program_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -p { nu -c 'exit 1' }; "text""#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "text");
}

#[test]
fn ignore_error_should_work_for_external_command() {
    let actual = nu!(r#"do -i { nu --testbin fail asdf }; echo post"#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "post");
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stderr_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stderr message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
            do -c {sh -c "cat a_large_file.txt 1>&2"} | complete | get stderr
            "#,
        ));

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stdout_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stdout message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
            do -c {sh -c "cat a_large_file.txt"} | complete | get stdout
            "#,
        ));

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_both_stdout_stderr_messages_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;

    Playground::setup(
        "external with many stdout and stderr messages",
        |dirs, sandbox| {
            let script_body = r#"
        x=$(printf '=%.0s' {1..40960})
        echo $x
        echo $x 1>&2
        "#;
            let mut expect_body = String::new();
            for _ in 0..40960 {
                expect_body.push('=');
            }

            sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);

            // check for stdout
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                "do -c {bash test.sh} | complete | get stdout | str trim",
            ));
            assert_eq!(actual.out, expect_body);
            // check for stderr
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                "do -c {bash test.sh} | complete | get stderr | str trim",
            ));
            assert_eq!(actual.out, expect_body);
        },
    )
}

#[test]
fn ignore_error_works_with_list_stream() {
    let actual = nu!(r#"do -i { ["a", $nothing, "b"] | ansi strip }"#);
    assert!(actual.err.is_empty());
}

#[test]
#[cfg(not(windows))] // windows requires too much effort to replicate permission errors
fn ignore_fs_related_errors() {
    use nu_test_support::{
        fs::{files_exist_at, Stub::EmptyFile},
        playground::Playground,
    };

    Playground::setup("ignore_fs_related_errors", |dirs, playground| {
        let file_names = vec!["test1.txt", "test2.txt", "test3.txt"];

        let files = file_names
            .iter()
            .map(|file_name| EmptyFile(file_name))
            .collect();

        playground.mkdir("subdir").with_files(files);

        let test_dir = dirs.test();
        let subdir = test_dir.join("subdir");
        let mut test_dir_permissions = playground.permissions(test_dir);
        let mut subdir_permissions = playground.permissions(&subdir);

        test_dir_permissions.set_readonly(true);
        subdir_permissions.set_readonly(true);
        test_dir_permissions.apply().unwrap();
        subdir_permissions.apply().unwrap();

        let actual = nu!(cwd: test_dir, "do -i { rm test*.txt }");

        assert!(
            actual.err.is_empty(),
            "`do -i {{ rm test*.txt }}` should ignore errors"
        );
        assert!(files_exist_at(file_names.clone(), test_dir));

        let actual = nu!(cwd: test_dir, "do -i { cp test*.txt subdir/ }");

        assert!(
            actual.err.is_empty(),
            "`do -i {{ cp test*.txt subdir/ }}` should ignore errors"
        );
        assert!(!files_exist_at(file_names.clone(), &subdir));

        let actual = nu!(cwd: test_dir, "do -i { mv test*.txt subdir/ }");

        assert!(
            actual.err.is_empty(),
            "do -i {{ mv test*.txt subdir/ }} should ignore errors"
        );
        assert!(!files_exist_at(file_names.clone(), &subdir));
    });
}
