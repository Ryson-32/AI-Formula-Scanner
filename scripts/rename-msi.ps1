# PowerShell script to rename MSI files after Tauri build
# Changes format from "AI Formula Scanner_x.x.x_x64_en-US.msi" to "AI Formula Scanner_x.x.x_x64-setup.msi"

$msiPath = "src-tauri/target/release/bundle/msi"

if (Test-Path $msiPath) {
    Write-Host "Looking for MSI files to rename in: $msiPath"
    
    # Find all MSI files with _en-US pattern
    $msiFiles = Get-ChildItem $msiPath -Filter "*_en-US.msi"
    
    if ($msiFiles.Count -eq 0) {
        Write-Host "No MSI files with '_en-US' pattern found."
        exit 0
    }
    
    foreach ($file in $msiFiles) {
        $oldName = $file.Name
        $newName = $oldName -replace "_en-US", "-setup"
        $oldPath = $file.FullName
        $newPath = Join-Path $file.Directory $newName
        
        Write-Host "Renaming: $oldName -> $newName"
        
        try {
            Rename-Item $oldPath $newPath
            Write-Host "✓ Successfully renamed to: $newName"
        }
        catch {
            Write-Error "✗ Failed to rename $oldName : $_"
        }
    }
} else {
    Write-Warning "MSI directory not found: $msiPath"
    Write-Host "Make sure you have run 'tauri build' first."
}
