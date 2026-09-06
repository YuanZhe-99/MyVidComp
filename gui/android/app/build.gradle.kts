// Imported rather than written out in full below: a build script has a `java`
// property of its own, which hides the package of the same name.
import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Written by the release workflow from the repository secrets, and by hand for
// a signed build made here. Without it a release build is signed with the debug
// key, which installs but can never be updated by a properly signed package.
val keystorePropertiesFile = rootProject.file("key.properties")
val keystoreProperties = Properties()
if (keystorePropertiesFile.exists()) {
    keystoreProperties.load(keystorePropertiesFile.inputStream())
}

android {
    namespace = "com.myvidcomp.app"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "com.myvidcomp.app"
        // Android 10 is where storage and process rules settled into the shape
        // this app relies on, and it still covers phones from 2019 onwards.
        minSdk = 29
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        // Architectures are chosen by the Flutter command line, with
        // --target-platform or --split-per-abi, so they are not pinned here.

    }

    packaging {
        jniLibs {
            // The conversion engine runs ffmpeg as a separate program, and
            // Android only allows that from the folder it unpacks native
            // libraries into. Legacy packaging is what makes it unpack them as
            // real files rather than mapping them straight out of the package.
            useLegacyPackaging = true
        }
    }

    signingConfigs {
        if (keystorePropertiesFile.exists()) {
            create("release") {
                storeFile = file(keystoreProperties["storeFile"]!!)
                storePassword = keystoreProperties["storePassword"] as String?
                keyAlias = keystoreProperties["keyAlias"] as String?
                keyPassword = keystoreProperties["keyPassword"] as String?
            }
        }
    }

    buildTypes {
        release {
            signingConfig = if (keystorePropertiesFile.exists()) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
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

/// Builds the Rust conversion engine for each Android architecture and drops
/// the results where Gradle expects to find native libraries.
val buildEngine = tasks.register<Exec>("buildConversionEngine") {
    workingDir = rootProject.projectDir.parentFile.parentFile
    val ndkDir = "${android.sdkDirectory}/ndk/${android.ndkVersion}"
    environment("ANDROID_NDK_HOME", ndkDir)
    environment("ANDROID_NDK_ROOT", ndkDir)

    val output = file("src/main/jniLibs").absolutePath
    val cargo = if (System.getProperty("os.name").lowercase().contains("win")) {
        "cargo.exe"
    } else {
        "cargo"
    }

    commandLine(
        cargo, "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-P", "29",
        "-o", output,
        "build", "--release",
    )

    // A missing Rust toolchain should say so plainly rather than failing deep
    // inside Gradle. The prebuilt libraries may also already be in place.
    isIgnoreExitValue = true
}

tasks.matching { it.name.startsWith("merge") && it.name.endsWith("JniLibFolders") }
    .configureEach { dependsOn(buildEngine) }
