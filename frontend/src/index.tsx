import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import AdminConfigurationPage from './pages/admin/AdminConfigurationPage.tsx';
import ModpacksPage from './pages/server/ModpacksPage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxModpacksExtension extends Extension {
  public cardConfigurationPage: React.FC | null = AdminConfigurationPage;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faCubes} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('pages.server.modpacks.title', {}),
        icon: faCubes,
        path: '/modpacks/*',
        element: ModpacksPage,
        permission: 'modpacks.read',
      }),
    );

    ctx.extensionRegistry.enterPermissionIcons((icons) =>
      icons
        .addServerPermissionIcon('modpacks', <FontAwesomeIcon icon={faCubes} />)
        .addAdminPermissionIcon('modpacks', <FontAwesomeIcon icon={faCubes} />),
    );
  }
}

export default new CaloptreyxModpacksExtension();
