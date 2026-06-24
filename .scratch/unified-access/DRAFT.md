The goal of goog cli is to provide a unified access experience for users. When users access resources with target (e.g., folder id, file id, doc id, etc.) It should try to resolve with the active account first. If it can't resolve, it must try to access with other accounts.
If an attempt succeed, the cli should remember the resource id pair with the account. Later, it can use that mapping to resolve the access quickly.


sainytk@Tanakorns-MacBook-Pro goog-cli % ./target/debug/goog drive list --folder 15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI
NAME    FILE ID PARENT FOLDER IDS       MIME TYPE       MODIFIED
Master transcript.pdf   172TAYDFxH2BV7KEBc8ULlMnBYmrjGoXa       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       application/pdf 2023-11-05T07:54:12.132Z
Master transcript.jpg   15Gnl7hKnssVrmOahUn0PreOq7uIjV3Zf       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:59.235Z
Bachelor transcript 2.jpg       15SaFEo_fQfJoPKItk1C3Nof_Hsd1NqLz       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:55.429Z
Bachelor transcript 1.jpg       15GYtjfo5WFZjYOxgxzDigbKaR4ylrq8G       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:47.235Z
Bachelor rank th.jpg    15JV_3PKQ6Mzxi-vha8TldPzdzEYI8ZuM       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:36.163Z
Bachelor rank.jpg       15Tgz158faOawEtji4swbKaN1_B3zoXId       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:27.964Z
Master Certificate.jpg  15Vw_Qc1Aeo348FWA4X_yWdgMC8NDxKsK       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       image/jpeg      2023-09-09T15:16:14.477Z
Bachelor Transcript.pdf 15bgigr_2Sh1Pl8xEyJjj0a5n3ripQat_       15BvTV2u65L8p1CbYJ9Ugrf1NXH9aLCuI       application/pdf 2023-09-09T15:15:49.413Z
sainytk@Tanakorns-MacBook-Pro goog-cli % ./target/debug/goog drive list --folder 10aIWCugPYA51qag23WNEMNSj2dxga8J5
NAME    FILE ID PARENT FOLDER IDS       MIME TYPE       MODIFIED