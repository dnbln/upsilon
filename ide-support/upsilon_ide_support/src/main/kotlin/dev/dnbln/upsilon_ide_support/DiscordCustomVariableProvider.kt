package dev.dnbln.upsilon_ide_support

import com.almightyalpaca.jetbrains.plugins.discord.plugin.data.CustomVariableData
import com.almightyalpaca.jetbrains.plugins.discord.plugin.extensions.CustomVariableProvider
import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile

object DiscordCustomVariableProvider : CustomVariableProvider {
    override fun forFile(variableData: CustomVariableData, project: Project, editor: FileEditor, file: VirtualFile) {
        val pathInProject = file.path.removePrefix(project.basePath ?: return)

        variableData["PathInProject"] = pathInProject

        val subCrateAndPath = pathInProject.removePrefix("/crates/upsilon-")

        variableData["UpsilonSubCratePath"] = subCrateAndPath
    }
}