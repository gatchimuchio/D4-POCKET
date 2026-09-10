plugins {
    id("com.android.application")
    // Flutterの固定順序に従ってpluginを適用する。
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.example.gui_shell_mobile"
    // 安全保管plugin 11の公開build要件。targetとminはFlutterの指定を保持する。
    compileSdk = 37
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // 開発用識別子。公開配布前に所有者が正式識別子と署名を確定する。
        applicationId = "com.example.gui_shell_mobile"
        // SDKの対応範囲は使用中のFlutter toolchainに従う。
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // releaseへ開発鍵を流用しない。正式署名は所有者による設定が必要。
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
