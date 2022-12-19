package dev.dnbln.upsilon_ide_support

import com.intellij.lang.injection.MultiHostInjector
import com.intellij.lang.injection.MultiHostRegistrar
import com.intellij.psi.PsiElement
import com.intellij.psi.util.parentOfType
import org.jetbrains.yaml.YAMLLanguage
import org.rust.lang.core.psi.*
import org.rust.lang.core.psi.ext.qualifiedName

class ConfigYAMLInjector : MultiHostInjector {
    override fun getLanguagesToInject(registrar: MultiHostRegistrar, context: PsiElement) {
        val config = context as? RsLitExpr ?: return

        val call = config.parentOfType<RsMethodCall>() ?: return

        if (call.valueArgumentList.exprList.firstOrNull() != config) {
            return
        }

        val resolved = call.reference.resolve() ?: return
        val resolvedFn = resolved as? RsFunction ?: return

        val qualifiedName = resolvedFn.qualifiedName

        if (qualifiedName !in setOf("upsilon_test_support::with_config"))
            return

        val range = (config.kind as? RsLiteralKind.String)?.offsets?.value ?: return

        registrar.startInjecting(YAMLLanguage.INSTANCE)
            .addPlace(null, null, config, range)
            .doneInjecting()
    }

    override fun elementsToInjectIn(): MutableList<out Class<out PsiElement>> = mutableListOf(RsLitExpr::class.java)
}